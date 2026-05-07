fn git_stdout(args: &[&str]) -> Option<String> {
    std::process::Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn main() {
    for git_path in [
        git_stdout(&["rev-parse", "--git-path", "HEAD"]),
        git_stdout(&["rev-parse", "--git-path", "index"]),
        git_stdout(&["rev-parse", "--git-path", "packed-refs"]),
    ]
    .into_iter()
    .flatten()
    {
        println!("cargo:rerun-if-changed={git_path}");
    }
    if let Some(head_ref) = git_stdout(&["symbolic-ref", "-q", "HEAD"])
        && let Some(ref_path) = git_stdout(&["rev-parse", "--git-path", &head_ref])
    {
        println!("cargo:rerun-if-changed={ref_path}");
    }
    let git_version = git_stdout(&["describe", "--tags", "--always", "--dirty"])
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=CYNIC_VERSION={git_version}");
}
