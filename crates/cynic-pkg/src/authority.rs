//! Principal authority observation port and local command adapter.

use std::process::Command;

pub trait PrincipalAuthorityPort: Send + Sync {
    fn current_username(&self) -> Option<String>;
    fn can_run_as(&self, user: &str) -> Option<bool>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalPrincipalAuthority;

impl PrincipalAuthorityPort for LocalPrincipalAuthority {
    fn current_username(&self) -> Option<String> {
        if let Ok(user) = std::env::var("USER")
            && !user.is_empty()
        {
            return Some(user);
        }
        let output = Command::new("id").args(["-un"]).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let user = String::from_utf8_lossy(&output.stdout).trim().to_string();
        (!user.is_empty()).then_some(user)
    }

    fn can_run_as(&self, user: &str) -> Option<bool> {
        Command::new("sudo")
            .args(["-n", "-u", user, "--", "true"])
            .output()
            .ok()
            .map(|output| output.status.success())
    }
}
