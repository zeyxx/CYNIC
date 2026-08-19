fn execute_release(root: &std::path::Path, _dry_run: bool, _apply: bool, json: bool) -> ExitCode {
    let observation = cynic_host::source_sync::observe_all_layers(root);
    let gate = cynic_host::release_bot::run_gate(root);

    let exit = if gate.all_passed() {
        SourceExit {
            status: "ready",
            summary: format!(
                "gate passed — forgejo ahead by {} commits, ready for release",
                observation.forgejo.commits_ahead.unwrap_or(0)
            ),
        }
    } else {
        SourceExit {
            status: "blocked",
            summary: format!(
                "gate failed — {} checks failed",
                gate.checks.iter().filter(|c| !c.passed).count()
            ),
        }
    };

    if json {
        emit_pretty_json(&exit, false);
    } else {
        println!("{}", exit.summary);
    }
    ExitCode::SUCCESS
}