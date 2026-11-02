//! Principal authority observation port and local command adapter.

use cynic_process::{ArgvRunner, LocalArgvRunner};

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
        let output = LocalArgvRunner.run(&["id", "-un"]);
        if !output.success() {
            return None;
        }
        let user = output.stdout.trim().to_string();
        (!user.is_empty()).then_some(user)
    }

    fn can_run_as(&self, user: &str) -> Option<bool> {
        let output = LocalArgvRunner.run(&["sudo", "-n", "-u", user, "--", "true"]);
        Some(output.success())
    }
}
