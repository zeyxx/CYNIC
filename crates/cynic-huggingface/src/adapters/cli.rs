//! argv-based adapter for the official Hugging Face `hf` CLI.

use std::process::Command;

use crate::domain::job::{HfJobInfo, HfJobKind, HfJobSpec, HfScheduledJobRequest};
use crate::domain::paths::HfLocalPaths;
use crate::port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort, HfJobLogsReport,
    HfJobReport, HfSearchRequest, HfUploadReport, HfUploadRequest,
};

#[derive(Debug)]
pub struct HfCliAdapter {
    bin: std::path::PathBuf,
}

impl HfCliAdapter {
    pub fn from_paths(paths: &HfLocalPaths) -> Self {
        Self {
            bin: paths.hf_bin.clone(),
        }
    }

    pub fn new(bin: impl Into<std::path::PathBuf>) -> Self {
        Self { bin: bin.into() }
    }

    fn run(&self, args: &[&str]) -> Result<(i32, String, String), HfHubError> {
        let output = Command::new(&self.bin).args(args).output().map_err(|e| {
            HfHubError::Unavailable(format!(
                "failed to spawn {}: {e} (install: curl -LsSf https://hf.co/cli/install.sh | bash -s)",
                self.bin.display()
            ))
        })?;
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((code, stdout, stderr))
    }

    fn tail(s: &str, max: usize) -> String {
        let t = s.trim();
        if t.len() <= max {
            return t.to_string();
        }
        t[t.len().saturating_sub(max)..].to_string()
    }

    /// Render `hf jobs [scheduled] ...` argv for a spec. Pure helper so the
    /// mapping stays testable without spawning anything.
    fn job_argv(scheduled: Option<&str>, spec: &HfJobSpec) -> Vec<String> {
        let mut argv = vec!["jobs".to_string()];
        if scheduled.is_some() {
            argv.push("scheduled".into());
        }
        match &spec.kind {
            HfJobKind::UvScript {
                script,
                script_args,
            } => {
                argv.push("uv".into());
                argv.push("run".into());
                if let Some(schedule) = scheduled {
                    argv.push(schedule.to_string());
                }
                Self::push_job_opts(&mut argv, spec);
                argv.push(script.clone());
                argv.extend(script_args.iter().cloned());
            }
            HfJobKind::Docker { image, command } => {
                argv.push("run".into());
                if let Some(schedule) = scheduled {
                    argv.push(schedule.to_string());
                }
                Self::push_job_opts(&mut argv, spec);
                argv.push(image.clone());
                argv.extend(command.iter().cloned());
            }
        }
        argv
    }

    fn push_job_opts(argv: &mut Vec<String>, spec: &HfJobSpec) {
        if let Some(name) = &spec.name {
            argv.push("--name".into());
            argv.push(name.clone());
        }
        if let Some(flavor) = &spec.flavor {
            argv.push("--flavor".into());
            argv.push(flavor.clone());
        }
        for (k, v) in &spec.env {
            argv.push("--env".into());
            argv.push(format!("{k}={v}"));
        }
        // By NAME: the CLI resolves values from the process environment.
        for secret in &spec.secret_names {
            argv.push("--secrets".into());
            argv.push(secret.clone());
        }
        for (k, v) in &spec.labels {
            argv.push("--label".into());
            argv.push(format!("{k}={v}"));
        }
        for vol in &spec.volumes {
            argv.push("--volume".into());
            argv.push(vol.clone());
        }
        if let Some(timeout) = &spec.timeout {
            argv.push("--timeout".into());
            argv.push(timeout.clone());
        }
        if let Some(ns) = &spec.namespace {
            argv.push("--namespace".into());
            argv.push(ns.clone());
        }
        if spec.detach {
            argv.push("--detach".into());
        }
    }

    fn namespaced(args: &mut Vec<String>, namespace: Option<&str>) {
        if let Some(ns) = namespace {
            args.push("--namespace".into());
            args.push(ns.to_string());
        }
    }

    fn to_job_report(
        &self,
        argv: &[String],
        code: i32,
        stdout: &str,
        stderr: &str,
        detached: bool,
    ) -> HfJobReport {
        let ok = code == 0;
        let remote_id = if ok && detached {
            let t = stdout.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        } else {
            None
        };
        HfJobReport {
            schema: HfJobReport::SCHEMA,
            surface: HfJobReport::SURFACE,
            ok,
            remote_id,
            argv: std::iter::once(self.bin.display().to_string())
                .chain(argv.iter().cloned())
                .collect(),
            stdout_tail: Self::tail(stdout, 2000),
            stderr_tail: Self::tail(stderr, 2000),
            message: if ok {
                Some("job submitted".into())
            } else {
                Some(format!("hf jobs exit {code}"))
            },
        }
    }

    fn parse_job_values(stdout: &str) -> Vec<serde_json::Value> {
        let Ok(val) = serde_json::from_str::<serde_json::Value>(stdout) else {
            return Vec::new();
        };
        match val {
            serde_json::Value::Array(arr) => arr,
            other => vec![other],
        }
    }
}

impl HfHubPort for HfCliAdapter {
    fn status(&self, paths: &HfLocalPaths) -> Result<HfCliStatus, HfHubError> {
        let (code, stdout, stderr) = match self.run(&["version", "--format", "json"]) {
            Ok(r) => r,
            Err(_) => self.run(&["version"])?,
        };
        if code != 0 {
            let (c2, o2, e2) = self.run(&["--version"]).unwrap_or((code, stdout, stderr));
            if c2 != 0 {
                return Ok(HfCliStatus {
                    schema: HfCliStatus::SCHEMA,
                    surface: HfCliStatus::SURFACE,
                    hf_bin: self.bin.display().to_string(),
                    available: false,
                    version: None,
                    whoami: None,
                    authenticated: false,
                    vault_dir: paths.vault_dir.display().to_string(),
                    hf_cache_dir: paths.hf_cache_dir.display().to_string(),
                    message: Some(format!("hf version failed: {}", e2.trim())),
                });
            }
            let version = o2.lines().next().map(|s| s.trim().to_string());
            return Ok(HfCliStatus {
                schema: HfCliStatus::SCHEMA,
                surface: HfCliStatus::SURFACE,
                hf_bin: self.bin.display().to_string(),
                available: true,
                version,
                whoami: None,
                authenticated: false,
                vault_dir: paths.vault_dir.display().to_string(),
                hf_cache_dir: paths.hf_cache_dir.display().to_string(),
                message: None,
            });
        }
        let version = serde_json::from_str::<serde_json::Value>(&stdout)
            .ok()
            .and_then(|v| {
                v.get("version")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                stdout
                    .lines()
                    .find_map(|l| l.split("version:").nth(1).map(|s| s.trim().to_string()))
            });

        let (w_code, w_out, _) = self.run(&["auth", "whoami", "--format", "json"])?;
        let (whoami, authenticated) = if w_code == 0 {
            let user = serde_json::from_str::<serde_json::Value>(&w_out)
                .ok()
                .and_then(|v| {
                    v.get("user")
                        .and_then(|u| u.as_str())
                        .map(|s| s.to_string())
                })
                .or_else(|| {
                    let t = w_out.trim().to_string();
                    if t.is_empty() { None } else { Some(t) }
                });
            (user, true)
        } else {
            (None, false)
        };

        Ok(HfCliStatus {
            schema: HfCliStatus::SCHEMA,
            surface: HfCliStatus::SURFACE,
            hf_bin: self.bin.display().to_string(),
            available: true,
            version,
            whoami,
            authenticated,
            vault_dir: paths.vault_dir.display().to_string(),
            hf_cache_dir: paths.hf_cache_dir.display().to_string(),
            message: None,
        })
    }

    fn download(
        &self,
        paths: &HfLocalPaths,
        req: &HfDownloadRequest,
    ) -> Result<HfDownloadReport, HfHubError> {
        if req.repo_id.trim().is_empty() {
            return Err(HfHubError::Rejected("repo_id required".into()));
        }

        let mut argv: Vec<String> = vec!["download".into(), req.repo_id.clone()];
        if let Some(rev) = &req.revision {
            argv.push("--revision".into());
            argv.push(rev.clone());
        }
        if let Some(t) = &req.repo_type {
            argv.push("--type".into());
            argv.push(t.clone());
        }
        for inc in &req.include {
            argv.push("--include".into());
            argv.push(inc.clone());
        }
        for exc in &req.exclude {
            argv.push("--exclude".into());
            argv.push(exc.clone());
        }

        let local_dir = if req.into_vault {
            let dir = paths.vault_download_dir(&req.repo_id);
            std::fs::create_dir_all(&dir).map_err(|e| {
                HfHubError::Command(format!("create vault dir {}: {e}", dir.display()))
            })?;
            argv.push("--local-dir".into());
            argv.push(dir.display().to_string());
            Some(dir.display().to_string())
        } else {
            None
        };

        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        let ok = code == 0;

        let primary_gguf = local_dir
            .as_ref()
            .and_then(|d| find_primary_gguf(std::path::Path::new(d)));

        Ok(HfDownloadReport {
            schema: HfDownloadReport::SCHEMA,
            surface: HfDownloadReport::SURFACE,
            ok,
            repo_id: req.repo_id.clone(),
            local_dir,
            argv: std::iter::once(self.bin.display().to_string())
                .chain(argv.iter().cloned())
                .collect(),
            stdout_tail: Self::tail(&stdout, 2000),
            stderr_tail: Self::tail(&stderr, 2000),
            message: if ok {
                Some("download completed into CYNIC vault (or hub cache)".into())
            } else {
                Some(format!("hf download exit {code}"))
            },
            primary_gguf,
            profile_alias: None,
            profile_path: None,
        })
    }

    fn search(&self, req: &HfSearchRequest) -> Result<Vec<serde_json::Value>, HfHubError> {
        let mut args: Vec<String> = vec![
            "models".into(),
            "list".into(),
            "--format".into(),
            "json".into(),
            "--limit".into(),
            req.limit.max(1).to_string(),
        ];
        if let Some(q) = &req.query
            && !q.is_empty()
        {
            args.push("--search".into());
            args.push(q.clone());
        }
        if let Some(a) = &req.author {
            args.push("--author".into());
            args.push(a.clone());
        }
        if req.gguf_only {
            args.push("--filter".into());
            args.push("gguf".into());
        }
        if let Some(np) = &req.num_parameters {
            args.push("--num-parameters".into());
            args.push(np.clone());
        }
        if let Some(sort) = &req.sort {
            args.push("--sort".into());
            args.push(sort.clone());
        } else {
            args.push("--sort".into());
            args.push("downloads".into());
        }
        // Expand useful fields when supported
        args.push("--expand".into());
        args.push("downloads,likes,tags,pipeline_tag,library_name,trendingScore,gguf".into());

        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        if code != 0 {
            return Err(HfHubError::Command(format!(
                "hf models list failed: {}",
                stderr.trim()
            )));
        }
        let val: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            HfHubError::Command(format!(
                "parse hf models list json: {e}; tail={}",
                Self::tail(&stdout, 400)
            ))
        })?;
        match val {
            serde_json::Value::Array(arr) => Ok(arr),
            other => Ok(vec![other]),
        }
    }

    fn list_repo_files(
        &self,
        repo_id: &str,
        revision: Option<&str>,
    ) -> Result<Vec<String>, HfHubError> {
        let mut args = vec![
            "models".to_string(),
            "list".to_string(),
            repo_id.to_string(),
            "-R".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        if let Some(r) = revision {
            args.push("--revision".into());
            args.push(r.to_string());
        }
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        if code != 0 {
            return Err(HfHubError::Command(format!(
                "hf models list {repo_id} failed: {}",
                stderr.trim()
            )));
        }
        // Output may be array of file objects or lines
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&stdout) {
            let mut files = Vec::new();
            if let serde_json::Value::Array(arr) = val {
                for item in arr {
                    if let Some(p) = item.get("path").and_then(|x| x.as_str()) {
                        files.push(p.to_string());
                    } else if let Some(p) = item.get("rfilename").and_then(|x| x.as_str()) {
                        files.push(p.to_string());
                    } else if let Some(s) = item.as_str() {
                        files.push(s.to_string());
                    }
                }
            }
            if !files.is_empty() {
                return Ok(files);
            }
        }
        Ok(stdout
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect())
    }

    fn upload(&self, req: &HfUploadRequest) -> Result<HfUploadReport, HfHubError> {
        if req.repo_id.trim().is_empty() {
            return Err(HfHubError::Rejected("repo_id required".into()));
        }
        if req.local_path.trim().is_empty() {
            return Err(HfHubError::Rejected("local_path required".into()));
        }
        // `hf upload REPO_ID LOCAL_PATH [PATH_IN_REPO] --type T ...`
        let mut argv: Vec<String> =
            vec!["upload".into(), req.repo_id.clone(), req.local_path.clone()];
        if let Some(p) = &req.path_in_repo {
            argv.push(p.clone());
        }
        argv.push("--type".into());
        argv.push(req.repo_type.as_arg().into());
        if let Some(rev) = &req.revision {
            argv.push("--revision".into());
            argv.push(rev.clone());
        }
        match req.private {
            Some(true) => argv.push("--private".into()),
            Some(false) => argv.push("--no-private".into()),
            None => {}
        }
        for inc in &req.include {
            argv.push("--include".into());
            argv.push(inc.clone());
        }
        for exc in &req.exclude {
            argv.push("--exclude".into());
            argv.push(exc.clone());
        }
        if let Some(msg) = &req.commit_message {
            argv.push("--commit-message".into());
            argv.push(msg.clone());
        }
        if req.create_pr {
            argv.push("--create-pr".into());
        }
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        let ok = code == 0;
        Ok(HfUploadReport {
            schema: HfUploadReport::SCHEMA,
            surface: HfUploadReport::SURFACE,
            ok,
            repo_id: req.repo_id.clone(),
            argv: std::iter::once(self.bin.display().to_string())
                .chain(argv.iter().cloned())
                .collect(),
            stdout_tail: Self::tail(&stdout, 2000),
            stderr_tail: Self::tail(&stderr, 2000),
            message: if ok {
                Some("upload completed".into())
            } else {
                Some(format!("hf upload exit {code}"))
            },
        })
    }

    fn job_submit(&self, spec: &HfJobSpec) -> Result<HfJobReport, HfHubError> {
        match &spec.kind {
            HfJobKind::UvScript { script, .. } if script.trim().is_empty() => {
                return Err(HfHubError::Rejected("uv script required".into()));
            }
            HfJobKind::Docker { image, .. } if image.trim().is_empty() => {
                return Err(HfHubError::Rejected("docker image required".into()));
            }
            _ => {}
        }
        let argv = Self::job_argv(None, spec);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        Ok(self.to_job_report(&argv, code, &stdout, &stderr, spec.detach))
    }

    fn job_poll(&self, remote_id: &str, namespace: Option<&str>) -> Result<HfJobInfo, HfHubError> {
        if remote_id.trim().is_empty() {
            return Err(HfHubError::Rejected("remote_id required".into()));
        }
        let mut argv = vec![
            "jobs".to_string(),
            "inspect".to_string(),
            remote_id.to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        Self::namespaced(&mut argv, namespace);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        if code != 0 {
            return Err(HfHubError::Command(format!(
                "hf jobs inspect {remote_id} failed: {}",
                stderr.trim()
            )));
        }
        Self::parse_job_values(&stdout)
            .iter()
            .find_map(HfJobInfo::from_raw)
            .ok_or_else(|| {
                HfHubError::Command(format!(
                    "unrecognized hf jobs inspect payload: {}",
                    Self::tail(&stdout, 400)
                ))
            })
    }

    fn job_list(&self, all: bool, namespace: Option<&str>) -> Result<Vec<HfJobInfo>, HfHubError> {
        let mut argv = vec![
            "jobs".to_string(),
            "ps".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        if all {
            argv.push("--all".into());
        }
        Self::namespaced(&mut argv, namespace);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        if code != 0 {
            return Err(HfHubError::Command(format!(
                "hf jobs ps failed: {}",
                stderr.trim()
            )));
        }
        Ok(Self::parse_job_values(&stdout)
            .iter()
            .filter_map(HfJobInfo::from_raw)
            .collect())
    }

    fn job_logs(
        &self,
        remote_id: &str,
        namespace: Option<&str>,
    ) -> Result<HfJobLogsReport, HfHubError> {
        if remote_id.trim().is_empty() {
            return Err(HfHubError::Rejected("remote_id required".into()));
        }
        let mut argv = vec![
            "jobs".to_string(),
            "logs".to_string(),
            remote_id.to_string(),
        ];
        Self::namespaced(&mut argv, namespace);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        let ok = code == 0;
        Ok(HfJobLogsReport {
            schema: HfJobLogsReport::SCHEMA,
            surface: HfJobLogsReport::SURFACE,
            ok,
            remote_id: remote_id.to_string(),
            logs: Self::tail(&stdout, 8000),
            message: if ok {
                None
            } else {
                Some(format!(
                    "hf jobs logs exit {code}: {}",
                    Self::tail(&stderr, 400)
                ))
            },
        })
    }

    fn job_cancel(
        &self,
        remote_id: &str,
        namespace: Option<&str>,
    ) -> Result<HfJobReport, HfHubError> {
        if remote_id.trim().is_empty() {
            return Err(HfHubError::Rejected("remote_id required".into()));
        }
        let mut argv = vec![
            "jobs".to_string(),
            "cancel".to_string(),
            remote_id.to_string(),
        ];
        Self::namespaced(&mut argv, namespace);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        Ok(self.to_job_report(&argv, code, &stdout, &stderr, false))
    }

    fn job_schedule_submit(&self, req: &HfScheduledJobRequest) -> Result<HfJobReport, HfHubError> {
        if req.schedule.trim().is_empty() {
            return Err(HfHubError::Rejected("schedule required".into()));
        }
        let argv = Self::job_argv(Some(req.schedule.as_str()), &req.spec);
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        Ok(self.to_job_report(&argv, code, &stdout, &stderr, true))
    }

    fn job_schedule_list(&self) -> Result<Vec<serde_json::Value>, HfHubError> {
        let argv = vec![
            "jobs".to_string(),
            "scheduled".to_string(),
            "ps".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        if code != 0 {
            return Err(HfHubError::Command(format!(
                "hf jobs scheduled ps failed: {}",
                stderr.trim()
            )));
        }
        Ok(Self::parse_job_values(&stdout))
    }

    fn job_schedule_delete(&self, scheduled_id: &str) -> Result<HfJobReport, HfHubError> {
        if scheduled_id.trim().is_empty() {
            return Err(HfHubError::Rejected("scheduled_id required".into()));
        }
        let argv = vec![
            "jobs".to_string(),
            "scheduled".to_string(),
            "delete".to_string(),
            scheduled_id.to_string(),
        ];
        let arg_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let (code, stdout, stderr) = self.run(&arg_refs)?;
        Ok(self.to_job_report(&argv, code, &stdout, &stderr, false))
    }
}

fn find_primary_gguf(dir: &std::path::Path) -> Option<String> {
    let mut best: Option<(u64, std::path::PathBuf)> = None;
    fn walk(dir: &std::path::Path, best: &mut Option<(u64, std::path::PathBuf)>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, best);
                continue;
            }
            if p.extension()
                .and_then(|x| x.to_str())
                .map(|x| x.eq_ignore_ascii_case("gguf"))
                != Some(true)
            {
                continue;
            }
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            match best {
                Some((s, _)) if size <= *s => {}
                _ => *best = Some((size, p)),
            }
        }
    }
    walk(dir, &mut best);
    best.map(|(_, p)| p.display().to_string())
}
