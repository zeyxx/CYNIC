//! HF discovery: category search, hardware ranking, profile registration.

use std::fs;
use std::path::Path;

use crate::adapters::hf::HfCliAdapter;
use crate::domain::hardware_profile::HardwareProfile;
use crate::domain::hf_catalog::{
    CategoryInfo, HardwareFit, ModelCategory, category_catalog, estimate_gguf_bytes, score_fit,
    short_alias_from_repo,
};
use crate::infra::model_layout::ModelLayout;
use crate::ports::hf_hub_port::{
    HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort, HfModelHit, HfSearchReport,
    HfSearchRequest,
};

#[derive(Debug)]
pub struct HfDiscoveryService {
    layout: ModelLayout,
    hf: HfCliAdapter,
}

#[derive(Debug, Clone)]
pub struct HfDownloadCommand {
    pub revision: Option<String>,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub repo_type: Option<String>,
    pub cache_only: bool,
    pub alias: Option<String>,
    pub register_profile: bool,
}

impl HfDownloadCommand {
    pub fn new(
        revision: Option<&str>,
        include: &[String],
        exclude: &[String],
        repo_type: Option<&str>,
        cache_only: bool,
        alias: Option<&str>,
        register_profile: bool,
    ) -> Self {
        Self {
            revision: revision.map(|s| s.to_string()),
            include: if include.is_empty() {
                vec!["*.gguf".into()]
            } else {
                include.to_vec()
            },
            exclude: exclude.to_vec(),
            repo_type: repo_type.map(|s| s.to_string()),
            cache_only,
            alias: alias.map(|s| s.to_string()),
            register_profile,
        }
    }
}

impl HfDiscoveryService {
    pub fn from_env() -> Self {
        let layout = ModelLayout::from_env();
        let hf = HfCliAdapter::from_layout(&layout);
        Self { layout, hf }
    }

    pub fn categories(&self) -> Vec<CategoryInfo> {
        category_catalog()
    }

    pub fn hardware(&self) -> HardwareProfile {
        crate::adapters::local::host_probe::probe_hardware()
    }

    pub fn search(
        &self,
        category: Option<&str>,
        query: Option<&str>,
        limit: u32,
        author: Option<&str>,
    ) -> Result<HfSearchReport, HfHubError> {
        let hw = crate::adapters::local::host_probe::probe_hardware();
        let cat = category.and_then(ModelCategory::parse);
        let search_q = match (cat, query) {
            (Some(c), q) => c.search_query(q),
            (None, Some(q)) => {
                if q.to_ascii_lowercase().contains("gguf") {
                    q.to_string()
                } else {
                    format!("{q} gguf")
                }
            }
            (None, None) => ModelCategory::Gguf.search_query(None),
        };

        let num_parameters = cat
            .and_then(|c| c.num_parameters_for_tier(&hw.tier).map(|s| s.to_string()))
            .or_else(|| match hw.tier.as_str() {
                "tiny" => Some("min:0B,max:3B".into()),
                "small" => Some("min:0B,max:8B".into()),
                "medium" => Some("min:1B,max:14B".into()),
                _ => None,
            });

        let filters_note = format!(
            "tier={} num_parameters={}",
            hw.tier,
            num_parameters.as_deref().unwrap_or("any")
        );

        let req = HfSearchRequest {
            query: if search_q.is_empty() {
                None
            } else {
                Some(search_q.clone())
            },
            category: cat.map(|c| c.as_str().to_string()),
            author: author.map(|s| s.to_string()),
            limit: limit.clamp(1, 50),
            sort: Some("downloads".into()),
            gguf_only: true,
            num_parameters,
        };

        let raw = match self.hf.search(&req) {
            Ok(r) if !r.is_empty() => r,
            Ok(_) | Err(_) => {
                // Retry without num-parameters if Hub rejects or returns empty
                let mut relaxed = req.clone();
                relaxed.num_parameters = None;
                self.hf.search(&relaxed)?
            }
        };
        let mut hits: Vec<HfModelHit> = raw.iter().filter_map(|v| map_hit(v, cat, &hw)).collect();

        hits.sort_by(|a, b| {
            b.fit_score
                .cmp(&a.fit_score)
                .then(b.downloads.unwrap_or(0).cmp(&a.downloads.unwrap_or(0)))
        });
        // Prefer comfortable fits first; keep some tight options
        let mut comfortable: Vec<_> = hits
            .iter()
            .filter(|h| h.hardware_fit == "comfortable")
            .cloned()
            .collect();
        let mut rest: Vec<_> = hits
            .into_iter()
            .filter(|h| h.hardware_fit != "comfortable")
            .collect();
        comfortable.append(&mut rest);
        comfortable.truncate(limit as usize);

        Ok(HfSearchReport {
            schema: HfSearchReport::SCHEMA,
            surface: HfSearchReport::SURFACE,
            query: search_q,
            category: cat.map(|c| c.as_str().to_string()),
            hardware_tier: hw.tier,
            recommended_max_weight_bytes: hw.recommended_max_weight_bytes,
            hits: comfortable,
            message: if filters_note.is_empty() {
                None
            } else {
                Some(filters_note)
            },
        })
    }

    pub fn suggest(
        &self,
        category: Option<&str>,
        limit: u32,
    ) -> Result<HfSearchReport, HfHubError> {
        let cat = category.unwrap_or("chat");
        self.search(Some(cat), None, limit, None)
    }

    pub fn download_and_register(
        &self,
        repo: &str,
        command: HfDownloadCommand,
    ) -> Result<HfDownloadReport, HfHubError> {
        let revision_for_profile = command.revision.clone();
        let req = HfDownloadRequest {
            repo_id: repo.to_string(),
            revision: revision_for_profile.clone(),
            include: command.include,
            exclude: command.exclude,
            into_vault: !command.cache_only,
            repo_type: command.repo_type,
        };
        let mut report = self.hf.download(&self.layout, &req)?;
        if !report.ok {
            return Ok(report);
        }

        let alias = command.alias.unwrap_or_else(|| short_alias_from_repo(repo));

        if command.register_profile {
            if let Some(ref gguf) = report.primary_gguf {
                match write_profile(
                    &self.layout,
                    &alias,
                    gguf,
                    repo,
                    revision_for_profile.as_deref(),
                ) {
                    Ok(path) => {
                        report.profile_alias = Some(alias);
                        report.profile_path = Some(path);
                        report.message = Some(
                            "download ok; native profile registered (loopback llama-server@)"
                                .into(),
                        );
                    }
                    Err(e) => {
                        report.message =
                            Some(format!("download ok; profile registration failed: {e}"));
                    }
                }
            } else {
                report.message =
                    Some("download ok; no GGUF found to register — pass --include '*.gguf'".into());
            }
        } else {
            report.profile_alias = Some(alias);
        }

        Ok(report)
    }
}

fn map_hit(
    v: &serde_json::Value,
    cat: Option<ModelCategory>,
    hw: &HardwareProfile,
) -> Option<HfModelHit> {
    let repo_id = v
        .get("id")
        .or_else(|| v.get("modelId"))
        .and_then(|x| x.as_str())?
        .to_string();
    let tags: Vec<String> = v
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let downloads = v.get("downloads").and_then(|x| x.as_u64());
    let likes = v.get("likes").and_then(|x| x.as_u64());
    let pipeline_tag = v
        .get("pipeline_tag")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let library_name = v
        .get("library_name")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let trending_score = v
        .get("trending_score")
        .or_else(|| v.get("trendingScore"))
        .and_then(|x| x.as_f64());

    let size = estimate_gguf_bytes(None, &tags).or_else(|| {
        // try gguf field from expand
        v.get("gguf")
            .and_then(|g| g.get("total"))
            .and_then(|x| x.as_u64())
            .or_else(|| {
                v.get("gguf")
                    .and_then(|g| g.get("size"))
                    .and_then(|x| x.as_u64())
            })
    });

    let (fit, fit_score) = score_fit(hw, size);
    let fit_s = match fit {
        HardwareFit::Comfortable => "comfortable",
        HardwareFit::Tight => "tight",
        HardwareFit::OverBudget => "over_budget",
        HardwareFit::Unknown => "unknown",
    }
    .to_string();

    Some(HfModelHit {
        repo_id: repo_id.clone(),
        alias: short_alias_from_repo(&repo_id),
        downloads,
        likes,
        pipeline_tag,
        library_name,
        tags,
        trending_score,
        estimated_size_bytes: size,
        hardware_fit: fit_s,
        fit_score,
        suggested_include: vec!["*.gguf".into(), "*Q4*.gguf".into(), "*Q5_K_M*.gguf".into()],
        category_hint: cat.map(|c| c.as_str().to_string()),
    })
}

fn write_profile(
    layout: &ModelLayout,
    alias: &str,
    gguf_path: &str,
    hf_repo: &str,
    revision: Option<&str>,
) -> Result<String, String> {
    fs::create_dir_all(&layout.profiles_dir).map_err(|e| format!("create profiles dir: {e}"))?;
    let path = layout.profiles_dir.join(format!("{alias}.env"));
    let rev = revision.unwrap_or("main");
    let body = format!(
        r#"# CYNIC native profile — auto from hf download
# hf_repo={hf_repo}
# hf_revision={rev}
LLAMA_SERVER_BIN=/usr/local/bin/llama-server
LLAMA_MODEL_PATH={gguf_path}
LLAMA_HOST=127.0.0.1
LLAMA_PORT=8080
LLAMA_CTX_SIZE=8192
LLAMA_EXTRA_ARGS=--parallel 1 -ctk q4_0 -ctv q4_0 --flash-attn on
LLAMA_MEMORY_MAX=8G
LLAMA_CPU_QUOTA=400%
CYNIC_MODEL_ALIAS={alias}
CYNIC_HF_REPO={hf_repo}
CYNIC_HF_REVISION={rev}
"#
    );
    fs::write(&path, body).map_err(|e| format!("write profile: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    // provenance sidecar next to gguf when possible
    if let Some(parent) = Path::new(gguf_path).parent() {
        let prov = parent.join(format!("{alias}.hf.json"));
        let _ = fs::write(
            &prov,
            serde_json::json!({
                "schema": "cynic.inference.hf_provenance.v1",
                "alias": alias,
                "hf_repo": hf_repo,
                "hf_revision": rev,
                "model_path": gguf_path,
            })
            .to_string(),
        );
    }
    Ok(path.display().to_string())
}
