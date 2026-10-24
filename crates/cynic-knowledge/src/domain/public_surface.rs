//! Public Surface Projection Engine
//!
//! Generates sovereign, zero-dependency Markdown and HTML projections
//! of the verified epistemic fact ledger (`cynic.facts.live.v1`).

use crate::domain::epistemic_facts::{
    ClaimStatus, EpistemicFactLedger, RepoEra, SkillBasis,
};

/// Renders the epistemic fact ledger into high-density GitHub-Flavored Markdown.
pub fn render_public_surface_markdown(ledger: &EpistemicFactLedger) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "# {} (@{})\n\n",
        ledger.identity.full_name, ledger.identity.handle
    ));
    out.push_str(&format!("> **{}**\n\n", ledger.identity.headline_en));
    out.push_str(&format!(
        "**Location**: {} | **GitHub**: [{}]({}) | **Email**: `{}`\n\n",
        ledger.identity.location,
        ledger.identity.github_login,
        ledger
            .identity
            .links
            .get("github")
            .cloned()
            .unwrap_or_default(),
        ledger.identity.email
    ));

    out.push_str("## Live Substrate Receipts (Verified Empirical State)\n\n");
    out.push_str("| Metric | Value | Verification SSOT |\n");
    out.push_str("|---|---|---|\n");
    for (k, v) in &ledger.summary_metrics {
        out.push_str(&format!("| `{}` | **{}** | Machine Measured |\n", k, v));
    }
    out.push_str("\n");

    out.push_str("## Method & Development Disclosure\n\n");
    out.push_str(&format!("{}\n\n", ledger.identity.disclosure_en));

    out.push_str("## The 4-Era GitHub Odyssey (35 Repositories)\n\n");
    for era in [
        RepoEra::NsiAwakening,
        RepoEra::OffensivePentest,
        RepoEra::SolanaTrench,
        RepoEra::SovereignRust,
    ] {
        out.push_str(&format!("### {}\n\n", era.label_en()));
        out.push_str("| Repository | Language | Domain | Authorship | Created | Description |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for r in ledger.repos.iter().filter(|r| r.era == era) {
            let priv_badge = if r.is_private { "🔒 `private`" } else { "🌐 `public`" };
            let authorship_badge = match r.authorship {
                crate::domain::epistemic_facts::AuthorshipNature::Original => "`original`".to_string(),
                crate::domain::epistemic_facts::AuthorshipNature::ContributedFork => {
                    format!("`co-developed fork` ({} commits)", r.author_commits)
                }
                crate::domain::epistemic_facts::AuthorshipNature::UpstreamAudit => {
                    let parent = r.parent_repo.as_deref().unwrap_or("upstream");
                    format!("`upstream audit` ({})", parent)
                }
            };
            out.push_str(&format!(
                "| [{}]({}) | `{}` | `{}` | {} | `{}` | {} — {} |\n",
                r.name,
                r.url,
                r.primary_language.as_deref().unwrap_or("None"),
                r.domain.as_str(),
                authorship_badge,
                &r.created_at[0..7.min(r.created_at.len())],
                priv_badge,
                r.description.replace('|', "\\|")
            ));
        }
        out.push_str("\n");
    }

    if let Some(graph) = &ledger.interactions {
        out.push_str("## GitHub Interactions & Ecosystem Graph\n\n");
        out.push_str(&format!(
            "> **Summary**: {} Pull Requests ({} external across 9 ecosystems, {} internal) & {} Issues ({} external security/compiler audits).\n\n",
            graph.total_prs,
            graph.external_prs,
            graph.internal_prs,
            graph.total_issues,
            graph.external_issues
        ));

        out.push_str("### High-Impact Security Bounties & Upstream Compiler Reports\n\n");
        out.push_str("| Category | Target Repository | Ref | Status | Discovery / Vulnerability Impact | Proof Receipt |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for item in graph.items.iter().filter(|i| {
            i.category == crate::domain::epistemic_facts::InteractionCategory::CompilerBug
                || i.category == crate::domain::epistemic_facts::InteractionCategory::SecurityBounty
        }) {
            let note = item.impact_note.as_deref().unwrap_or(item.title.as_str());
            out.push_str(&format!(
                "| `{}` | `{}` | `#{}` | `{}` | {} | [Proof Link]({}) |\n",
                item.category.badge_label(),
                item.repository,
                item.number,
                item.state,
                note.replace('|', "\\|"),
                item.url
            ));
        }
        out.push_str("\n");

        out.push_str("### Ecosystem Protocol Contributions & Co-Development\n\n");
        out.push_str("| Category | Target Repository | Ref | Status | Contribution Title | Link |\n");
        out.push_str("|---|---|---|---|---|---|\n");
        for item in graph.items.iter().filter(|i| {
            i.is_external
                && i.category != crate::domain::epistemic_facts::InteractionCategory::CompilerBug
                && i.category != crate::domain::epistemic_facts::InteractionCategory::SecurityBounty
        }) {
            out.push_str(&format!(
                "| `{}` | `{}` | `#{}` | `{}` | {} | [View]({}) |\n",
                item.category.badge_label(),
                item.repository,
                item.number,
                item.state,
                item.title.replace('|', "\\|"),
                item.url
            ));
        }
        out.push_str("\n");
    }

    out.push_str("## Calibrated Competencies (Three-Tier Grounding)\n\n");
    out.push_str("All skills are calibrated by who actually executes the code:\n");
    out.push_str("- `exercised` : directly authored and executed by the human\n");
    out.push_str("- `understood` : conceptually understood and steered through AI coding agents\n");
    out.push_str("- `exposure` : surveyed and followed without claimed manual mastery\n\n");

    for group in &ledger.skills {
        let depth_str = group
            .depth
            .map(|d| format!(" · {}", d.label_en()))
            .unwrap_or_default();
        out.push_str(&format!(
            "### {} `({}{})`\n\n",
            group.label_en,
            group.basis.label_en(),
            depth_str
        ));
        if let Some(note) = &group.note_en {
            out.push_str(&format!("_{}_\n\n", note));
        }
        for item in &group.items_en {
            out.push_str(&format!(
                "- **[{}]** {}\n",
                item.basis.label_en(),
                item.text
            ));
        }
        out.push_str("\n");
    }

    out.push_str("## Verifiable Claims Ledger\n\n");
    out.push_str("| Status | Claim ID | Assertion | Physical Evidence | Verification Command |\n");
    out.push_str("|---|---|---|---|---|\n");
    for claim in &ledger.claims {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | `{}` | `{}` |\n",
            claim.status.badge_label(),
            claim.id,
            claim.statement.replace('|', "\\|"),
            claim.evidence.replace('|', "\\|"),
            claim.verify_command.as_deref().unwrap_or("n/a").replace('|', "\\|")
        ));
    }
    out.push_str("\n");

    out.push_str("---\n");
    out.push_str(&format!(
        "_Generated by CYNIC Sovereign Epistemic Engine (`{}`) at {}_\n",
        ledger.schema, ledger.generated_at
    ));

    out
}

/// Renders the epistemic fact ledger into a standalone, ultra-fast, dark terminal HTML surface.
pub fn render_public_surface_html(ledger: &EpistemicFactLedger) -> String {
    let mut html = String::new();

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html lang=\"en\">\n<head>\n");
    html.push_str("  <meta charset=\"UTF-8\" />\n");
    html.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n");
    html.push_str(&format!(
        "  <title>{} (@{}) — Sovereign Epistemic Surface</title>\n",
        ledger.identity.full_name, ledger.identity.handle
    ));
    html.push_str("  <style>\n");
    html.push_str(
        r#"
    :root {
      --bg: #070709;
      --card-bg: #0e0e13;
      --border: #1c1c24;
      --border-accent: #2e303e;
      --text: #e2e8f0;
      --text-muted: #94a3b8;
      --text-dim: #64748b;
      --accent: #10b981;
      --cyan: #38bdf8;
      --amber: #f59e0b;
      --purple: #a855f7;
      --code-bg: #14141c;
      --font-mono: 'JetBrains Mono', 'Fira Code', 'SF Mono', Consolas, monospace;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      background-color: var(--bg);
      color: var(--text);
      font-family: var(--font-mono);
      font-size: 13px;
      line-height: 1.6;
      padding: 24px 16px;
    }
    .container {
      max-width: 1080px;
      margin: 0 auto;
    }
    header {
      border-bottom: 1px solid var(--border);
      padding-bottom: 24px;
      margin-bottom: 28px;
    }
    .hud-title {
      font-size: 11px;
      letter-spacing: 0.15em;
      text-transform: uppercase;
      color: var(--accent);
      margin-bottom: 8px;
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .hud-title::before {
      content: "";
      display: inline-block;
      width: 8px;
      height: 8px;
      background: var(--accent);
      border-radius: 50%;
      box-shadow: 0 0 10px var(--accent);
    }
    h1 {
      font-size: 26px;
      font-weight: 700;
      letter-spacing: -0.02em;
      color: #fff;
      margin-bottom: 8px;
    }
    .headline {
      font-size: 14px;
      color: var(--cyan);
      margin-bottom: 16px;
    }
    .meta-bar {
      display: flex;
      flex-wrap: wrap;
      gap: 16px;
      font-size: 12px;
      color: var(--text-muted);
    }
    .meta-bar a {
      color: var(--text);
      text-decoration: underline;
    }
    .metrics-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
      gap: 12px;
      margin: 24px 0 36px 0;
    }
    .metric-card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 14px 16px;
      transition: border-color 0.15s ease;
    }
    .metric-card:hover {
      border-color: var(--border-accent);
    }
    .metric-value {
      font-size: 20px;
      font-weight: 700;
      color: #fff;
    }
    .metric-label {
      font-size: 11px;
      color: var(--text-dim);
      text-transform: uppercase;
      letter-spacing: 0.05em;
      margin-top: 4px;
    }
    section {
      margin-bottom: 40px;
    }
    h2 {
      font-size: 16px;
      font-weight: 700;
      color: #fff;
      letter-spacing: -0.01em;
      margin-bottom: 16px;
      display: flex;
      align-items: center;
      gap: 8px;
      border-left: 3px solid var(--accent);
      padding-left: 10px;
    }
    .disclosure-box {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-left: 3px solid var(--purple);
      padding: 16px 20px;
      border-radius: 6px;
      margin-bottom: 24px;
      color: var(--text-muted);
      font-size: 12.5px;
    }
    .badge {
      display: inline-block;
      padding: 2px 8px;
      font-size: 10px;
      font-weight: 600;
      border-radius: 4px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }
    .badge-demonstrated { background: rgba(16, 185, 129, 0.15); color: #10b981; border: 1px solid rgba(16, 185, 129, 0.3); }
    .badge-implemented { background: rgba(56, 189, 248, 0.15); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.3); }
    .badge-partial { background: rgba(245, 158, 11, 0.15); color: #f59e0b; border: 1px solid rgba(245, 158, 11, 0.3); }
    .badge-aspirational { background: rgba(168, 85, 247, 0.15); color: #a855f7; border: 1px solid rgba(168, 85, 247, 0.3); }
    .badge-basis-exercised { background: rgba(16, 185, 129, 0.15); color: #10b981; }
    .badge-basis-understood { background: rgba(56, 189, 248, 0.15); color: #38bdf8; }
    .badge-basis-exposure { background: rgba(148, 163, 184, 0.15); color: #94a3b8; }
    .badge-compiler { background: rgba(232, 121, 249, 0.15); color: #e879f9; border: 1px solid rgba(232, 121, 249, 0.3); }
    .badge-bounty { background: rgba(244, 63, 94, 0.15); color: #f43f5e; border: 1px solid rgba(244, 63, 94, 0.3); }
    .badge-protocol { background: rgba(56, 189, 248, 0.15); color: #38bdf8; border: 1px solid rgba(56, 189, 248, 0.3); }
    .badge-partner { background: rgba(52, 211, 153, 0.15); color: #34d399; border: 1px solid rgba(52, 211, 153, 0.3); }
    .badge-governance { background: rgba(148, 163, 184, 0.15); color: #94a3b8; border: 1px solid rgba(148, 163, 184, 0.3); }
    .badge-state-merged { background: rgba(168, 85, 247, 0.15); color: #c084fc; border: 1px solid rgba(168, 85, 247, 0.3); }
    .badge-state-open { background: rgba(16, 185, 129, 0.15); color: #10b981; border: 1px solid rgba(16, 185, 129, 0.3); }
    .badge-state-closed { background: rgba(100, 116, 139, 0.15); color: #94a3b8; border: 1px solid rgba(100, 116, 139, 0.3); }
    
    .timeline-era {
      margin-bottom: 24px;
    }
    .era-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--cyan);
      margin-bottom: 10px;
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .repos-table, .claims-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 12px;
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      overflow: hidden;
    }
    .repos-table th, .repos-table td,
    .claims-table th, .claims-table td {
      padding: 10px 14px;
      text-align: left;
      border-bottom: 1px solid var(--border);
    }
    .repos-table th, .claims-table th {
      background: #111117;
      color: var(--text-dim);
      font-weight: 600;
      text-transform: uppercase;
      font-size: 10px;
      letter-spacing: 0.05em;
    }
    .repos-table tr:hover, .claims-table tr:hover {
      background: rgba(255, 255, 255, 0.02);
    }
    .repo-link {
      color: #fff;
      text-decoration: none;
      font-weight: 600;
    }
    .repo-link:hover { text-decoration: underline; color: var(--accent); }
    
    .skills-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 16px;
    }
    .skill-card {
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 6px;
      padding: 16px;
    }
    .skill-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 10px;
    }
    .skill-name {
      font-size: 13px;
      font-weight: 600;
      color: #fff;
    }
    .skill-items {
      list-style: none;
    }
    .skill-items li {
      margin-bottom: 6px;
      color: var(--text-muted);
      font-size: 12px;
      display: flex;
      align-items: baseline;
      gap: 8px;
    }
    .code-receipt {
      background: var(--code-bg);
      padding: 2px 6px;
      border-radius: 4px;
      font-size: 11px;
      color: #cbd5e1;
      border: 1px solid var(--border);
    }
    footer {
      border-top: 1px solid var(--border);
      padding-top: 20px;
      margin-top: 40px;
      font-size: 11px;
      color: var(--text-dim);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
  "#,
    );
    html.push_str("  </style>\n</head>\n<body>\n");
    html.push_str("  <div class=\"container\">\n");

    // Header
    html.push_str("    <header>\n");
    html.push_str("      <div class=\"hud-title\">CYNIC Epistemic Engine &bull; Sovereign Surface</div>\n");
    html.push_str(&format!(
        "      <h1>{} <span style=\"color: var(--text-dim); font-weight: 400;\">(@{})</span></h1>\n",
        ledger.identity.full_name, ledger.identity.handle
    ));
    html.push_str(&format!(
        "      <div class=\"headline\">{}</div>\n",
        ledger.identity.headline_en
    ));
    html.push_str("      <div class=\"meta-bar\">\n");
    html.push_str(&format!("        <span>📍 {}</span>\n", ledger.identity.location));
    html.push_str(&format!(
        "        <span>🔗 <a href=\"https://github.com/{}\" target=\"_blank\">github.com/{}</a></span>\n",
        ledger.identity.github_login, ledger.identity.github_login
    ));
    html.push_str(&format!("        <span>✉️ {}</span>\n", ledger.identity.email));
    html.push_str(&format!(
        "        <span>⏱️ Generated: {}</span>\n",
        ledger.generated_at
    ));
    html.push_str("      </div>\n");
    html.push_str("    </header>\n\n");

    // Metrics HUD
    html.push_str("    <div class=\"metrics-grid\">\n");
    for (k, v) in &ledger.summary_metrics {
        let label = match k.as_str() {
            "cynic_loc" => "CYNIC Rust LOC",
            "cynic_crates" => "Crates in DAG",
            "total_commits" => "VCS Commits",
            "git_add_all_violations" => "git add . violations",
            "active_days" => "Active Coding Days",
            "test_attributes" => "Verified Tests",
            "contracts" => "JSON Contracts",
            "github_repos" => "Total Repositories",
            "total_prs" => "GitHub PRs",
            "external_prs" => "External PRs",
            "external_issues" => "Security & Bugs",
            "external_ecosystems" => "Ecosystems",
            _ => k.as_str(),
        };
        html.push_str("      <div class=\"metric-card\">\n");
        html.push_str(&format!("        <div class=\"metric-value\">{}</div>\n", v));
        html.push_str(&format!("        <div class=\"metric-label\">{}</div>\n", label));
        html.push_str("      </div>\n");
    }
    html.push_str("    </div>\n\n");

    // Method & Disclosure
    html.push_str("    <section>\n");
    html.push_str("      <h2>Method & Development Disclosure</h2>\n");
    html.push_str("      <div class=\"disclosure-box\">\n");
    html.push_str(&format!("        <p>{}</p>\n", ledger.identity.disclosure_en));
    html.push_str("      </div>\n");
    html.push_str("    </section>\n\n");

    // The 4-Era GitHub Odyssey
    html.push_str("    <section>\n");
    html.push_str("      <h2>The 4-Era GitHub Odyssey (35 Repositories)</h2>\n");
    for era in [
        RepoEra::NsiAwakening,
        RepoEra::OffensivePentest,
        RepoEra::SolanaTrench,
        RepoEra::SovereignRust,
    ] {
        html.push_str("      <div class=\"timeline-era\">\n");
        html.push_str(&format!(
            "        <div class=\"era-title\">🔹 {}</div>\n",
            era.label_en()
        ));
        html.push_str("        <table class=\"repos-table\">\n");
        html.push_str("          <thead>\n");
        html.push_str("            <tr>\n");
        html.push_str("              <th>Repository</th>\n");
        html.push_str("              <th>Language</th>\n");
        html.push_str("              <th>Domain</th>\n");
        html.push_str("              <th>Authorship / Origin</th>\n");
        html.push_str("              <th>Created</th>\n");
        html.push_str("              <th>Description</th>\n");
        html.push_str("            </tr>\n");
        html.push_str("          </thead>\n");
        html.push_str("          <tbody>\n");
        for r in ledger.repos.iter().filter(|r| r.era == era) {
            let lang = r.primary_language.as_deref().unwrap_or("—");
            let date = &r.created_at[0..7.min(r.created_at.len())];
            let authorship_badge = match r.authorship {
                crate::domain::epistemic_facts::AuthorshipNature::Original => {
                    "<span class=\"badge badge-demonstrated\">Original</span>".to_string()
                }
                crate::domain::epistemic_facts::AuthorshipNature::ContributedFork => {
                    format!("<span class=\"badge badge-implemented\">Co-Dev ({} commits)</span>", r.author_commits)
                }
                crate::domain::epistemic_facts::AuthorshipNature::UpstreamAudit => {
                    let parent = r.parent_repo.as_deref().unwrap_or("upstream");
                    format!("<span class=\"badge badge-partial\">Audit ({})</span>", parent)
                }
            };
            html.push_str("            <tr>\n");
            html.push_str(&format!(
                "              <td><a class=\"repo-link\" href=\"{}\" target=\"_blank\">{}</a> {}</td>\n",
                r.url,
                r.name,
                if r.is_private { "🔒" } else { "" }
            ));
            html.push_str(&format!("              <td><code>{}</code></td>\n", lang));
            html.push_str(&format!("              <td><code>{}</code></td>\n", r.domain.as_str()));
            html.push_str(&format!("              <td>{}</td>\n", authorship_badge));
            html.push_str(&format!("              <td><code>{}</code></td>\n", date));
            html.push_str(&format!("              <td>{}</td>\n", r.description));
            html.push_str("            </tr>\n");
        }
        html.push_str("          </tbody>\n");
        html.push_str("        </table>\n");
        html.push_str("      </div>\n");
    }
    html.push_str("    </section>\n\n");

    // GitHub Interactions & Ecosystem Graph
    if let Some(graph) = &ledger.interactions {
        html.push_str("    <section>\n");
        html.push_str(&format!(
            "      <h2>GitHub Interactions & Ecosystem Graph ({} PRs &bull; {} Issues &bull; {} Ecosystems)</h2>\n",
            graph.total_prs, graph.total_issues, graph.external_ecosystems.len()
        ));

        // Sub-table 1: Security Bounties & Compiler Reports
        html.push_str("      <div class=\"timeline-era\">\n");
        html.push_str("        <div class=\"era-title\" style=\"color: #f43f5e;\">🛡️ High-Impact Security Audits & Compiler Bugs</div>\n");
        html.push_str("        <table class=\"repos-table\">\n");
        html.push_str("          <thead>\n");
        html.push_str("            <tr>\n");
        html.push_str("              <th>Category</th>\n");
        html.push_str("              <th>Target Repository</th>\n");
        html.push_str("              <th>Ref</th>\n");
        html.push_str("              <th>Status</th>\n");
        html.push_str("              <th>Discovery / Vulnerability Impact</th>\n");
        html.push_str("            </tr>\n");
        html.push_str("          </thead>\n");
        html.push_str("          <tbody>\n");
        for item in graph.items.iter().filter(|i| {
            i.category == crate::domain::epistemic_facts::InteractionCategory::CompilerBug
                || i.category == crate::domain::epistemic_facts::InteractionCategory::SecurityBounty
        }) {
            let note = item.impact_note.as_deref().unwrap_or(item.title.as_str());
            let state_class = match item.state.as_str() {
                "merged" => "badge-state-merged",
                "open" => "badge-state-open",
                _ => "badge-state-closed",
            };
            html.push_str("            <tr>\n");
            html.push_str(&format!(
                "              <td><span class=\"badge {}\">{}</span></td>\n",
                item.category.css_class(),
                item.category.badge_label()
            ));
            html.push_str(&format!("              <td><code>{}</code></td>\n", item.repository));
            html.push_str(&format!(
                "              <td><a class=\"repo-link\" href=\"{}\" target=\"_blank\">#{}</a></td>\n",
                item.url, item.number
            ));
            html.push_str(&format!(
                "              <td><span class=\"badge {}\">{}</span></td>\n",
                state_class, item.state
            ));
            html.push_str(&format!("              <td>{}</td>\n", note));
            html.push_str("            </tr>\n");
        }
        html.push_str("          </tbody>\n");
        html.push_str("        </table>\n");
        html.push_str("      </div>\n\n");

        // Sub-table 2: External Protocol Contributions
        html.push_str("      <div class=\"timeline-era\">\n");
        html.push_str("        <div class=\"era-title\">🌐 External Protocol Contributions & Ecosystem PRs</div>\n");
        html.push_str("        <table class=\"repos-table\">\n");
        html.push_str("          <thead>\n");
        html.push_str("            <tr>\n");
        html.push_str("              <th>Category</th>\n");
        html.push_str("              <th>Target Repository</th>\n");
        html.push_str("              <th>Ref</th>\n");
        html.push_str("              <th>Status</th>\n");
        html.push_str("              <th>Contribution Title</th>\n");
        html.push_str("            </tr>\n");
        html.push_str("          </thead>\n");
        html.push_str("          <tbody>\n");
        for item in graph.items.iter().filter(|i| {
            i.is_external
                && i.category != crate::domain::epistemic_facts::InteractionCategory::CompilerBug
                && i.category != crate::domain::epistemic_facts::InteractionCategory::SecurityBounty
        }) {
            let state_class = match item.state.as_str() {
                "merged" => "badge-state-merged",
                "open" => "badge-state-open",
                _ => "badge-state-closed",
            };
            html.push_str("            <tr>\n");
            html.push_str(&format!(
                "              <td><span class=\"badge {}\">{}</span></td>\n",
                item.category.css_class(),
                item.category.badge_label()
            ));
            html.push_str(&format!("              <td><code>{}</code></td>\n", item.repository));
            html.push_str(&format!(
                "              <td><a class=\"repo-link\" href=\"{}\" target=\"_blank\">#{}</a></td>\n",
                item.url, item.number
            ));
            html.push_str(&format!(
                "              <td><span class=\"badge {}\">{}</span></td>\n",
                state_class, item.state
            ));
            html.push_str(&format!("              <td>{}</td>\n", item.title));
            html.push_str("            </tr>\n");
        }
        html.push_str("          </tbody>\n");
        html.push_str("        </table>\n");
        html.push_str("      </div>\n");

        html.push_str("    </section>\n\n");
    }

    // Calibrated Skills
    html.push_str("    <section>\n");
    html.push_str("      <h2>Calibrated Competencies Matrix</h2>\n");
    html.push_str("      <div class=\"skills-grid\">\n");
    for group in &ledger.skills {
        let badge_class = match group.basis {
            SkillBasis::Exercised => "badge-basis-exercised",
            SkillBasis::Understood => "badge-basis-understood",
            SkillBasis::Exposition => "badge-basis-exposure",
        };
        html.push_str("        <div class=\"skill-card\">\n");
        html.push_str("          <div class=\"skill-header\">\n");
        html.push_str(&format!(
            "            <div class=\"skill-name\">{}</div>\n",
            group.label_en
        ));
        html.push_str(&format!(
            "            <span class=\"badge {}\">{}</span>\n",
            badge_class,
            group.basis.label_en()
        ));
        html.push_str("          </div>\n");
        if let Some(note) = &group.note_en {
            html.push_str(&format!(
                "          <div style=\"font-size: 11px; color: var(--text-dim); margin-bottom: 10px;\">{}</div>\n",
                note
            ));
        }
        html.push_str("          <ul class=\"skill-items\">\n");
        for item in &group.items_en {
            let item_badge = match item.basis {
                SkillBasis::Exercised => "<span class=\"badge badge-basis-exercised\" style=\"font-size: 9px;\">exercised</span>",
                SkillBasis::Understood => "<span class=\"badge badge-basis-understood\" style=\"font-size: 9px;\">understood</span>",
                SkillBasis::Exposition => "<span class=\"badge badge-basis-exposure\" style=\"font-size: 9px;\">exposure</span>",
            };
            html.push_str(&format!(
                "            <li>{} <span>{}</span></li>\n",
                item_badge, item.text
            ));
        }
        html.push_str("          </ul>\n");
        html.push_str("        </div>\n");
    }
    html.push_str("      </div>\n");
    html.push_str("    </section>\n\n");

    // Verifiable Claims Ledger
    html.push_str("    <section>\n");
    html.push_str("      <h2>Verifiable Claims Ledger (Physical Receipts)</h2>\n");
    html.push_str("      <table class=\"claims-table\">\n");
    html.push_str("        <thead>\n");
    html.push_str("          <tr>\n");
    html.push_str("            <th>Status</th>\n");
    html.push_str("            <th>Claim ID</th>\n");
    html.push_str("            <th>Assertion</th>\n");
    html.push_str("            <th>Physical Evidence</th>\n");
    html.push_str("            <th>Deterministic Command</th>\n");
    html.push_str("          </tr>\n");
    html.push_str("        </thead>\n");
    html.push_str("        <tbody>\n");
    for claim in &ledger.claims {
        let badge_class = match claim.status {
            ClaimStatus::Demonstrated => "badge-demonstrated",
            ClaimStatus::Implemented => "badge-implemented",
            ClaimStatus::Partial => "badge-partial",
            ClaimStatus::Aspirational => "badge-aspirational",
            ClaimStatus::Unknown => "badge-basis-exposure",
        };
        let cmd = claim
            .verify_command
            .as_deref()
            .map(|c| format!("<code class=\"code-receipt\">{}</code>", c))
            .unwrap_or_else(|| "<span style=\"color: var(--text-dim);\">n/a</span>".to_string());

        html.push_str("          <tr>\n");
        html.push_str(&format!(
            "            <td><span class=\"badge {}\">{}</span></td>\n",
            badge_class,
            claim.status.badge_label()
        ));
        html.push_str(&format!("            <td><code>{}</code></td>\n", claim.id));
        html.push_str(&format!("            <td>{}</td>\n", claim.statement));
        html.push_str(&format!("            <td><code class=\"code-receipt\">{}</code></td>\n", claim.evidence));
        html.push_str(&format!("            <td>{}</td>\n", cmd));
        html.push_str("          </tr>\n");
    }
    html.push_str("        </tbody>\n");
    html.push_str("      </table>\n");
    html.push_str("    </section>\n\n");

    // Footer
    html.push_str("    <footer>\n");
    html.push_str(&format!(
        "      <div>Schema: <code>{}</code> &bull; Pure Zero-I/O Rust Engine</div>\n",
        ledger.schema
    ));
    html.push_str("      <div>Grounded in physical receipts. No mutation without read-back proof.</div>\n");
    html.push_str("    </footer>\n");

    html.push_str("  </div>\n");
    html.push_str("</body>\n</html>\n");

    html
}
