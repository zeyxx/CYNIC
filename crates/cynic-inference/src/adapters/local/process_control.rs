//! OS process observation and bounded termination adapters.

use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::ports::local_runtime_port::LocalRuntimeError;

pub trait ProcessControlPort: Send + Sync {
    fn is_running(&self, pid: u32) -> bool;
    fn terminate(&self, pid: u32) -> Result<(), LocalRuntimeError>;
}

#[derive(Debug, Default)]
pub struct SystemProcessControl;

#[cfg(unix)]
impl ProcessControlPort for SystemProcessControl {
    fn is_running(&self, pid: u32) -> bool {
        pid > 0
            && Command::new("kill")
                .args(["-0", &pid.to_string()])
                .status()
                .is_ok_and(|status| status.success())
    }

    fn terminate(&self, pid: u32) -> Result<(), LocalRuntimeError> {
        let term = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .map_err(|error| LocalRuntimeError::Control(format!("terminate pid {pid}: {error}")))?;
        if !term.success() && self.is_running(pid) {
            return Err(LocalRuntimeError::Control(format!(
                "terminate pid {pid}: kill -TERM exited {term}"
            )));
        }
        thread::sleep(Duration::from_millis(400));
        if self.is_running(pid) {
            let kill = Command::new("kill")
                .args(["-KILL", &pid.to_string()])
                .status()
                .map_err(|error| {
                    LocalRuntimeError::Control(format!("force terminate pid {pid}: {error}"))
                })?;
            if !kill.success() || self.is_running(pid) {
                return Err(LocalRuntimeError::Control(format!(
                    "pid {pid} remained alive after bounded termination"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn observes_current_process_without_mutation() {
        assert!(SystemProcessControl.is_running(std::process::id()));
    }
}

#[cfg(windows)]
impl ProcessControlPort for SystemProcessControl {
    fn is_running(&self, pid: u32) -> bool {
        if pid == 0 {
            return false;
        }
        Command::new("tasklist.exe")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .is_some_and(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .split(',')
                    .nth(1)
                    .is_some_and(|field| field.trim_matches([' ', '"']) == pid.to_string())
            })
    }

    fn terminate(&self, pid: u32) -> Result<(), LocalRuntimeError> {
        let _status = Command::new("taskkill.exe")
            .args(["/PID", &pid.to_string(), "/T"])
            .status()
            .map_err(|error| LocalRuntimeError::Control(format!("terminate pid {pid}: {error}")))?;
        thread::sleep(Duration::from_millis(400));
        if self.is_running(pid) {
            let forced = Command::new("taskkill.exe")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status()
                .map_err(|error| {
                    LocalRuntimeError::Control(format!("force terminate pid {pid}: {error}"))
                })?;
            if !forced.success() || self.is_running(pid) {
                return Err(LocalRuntimeError::Control(format!(
                    "pid {pid} remained alive after bounded termination"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
impl ProcessControlPort for SystemProcessControl {
    fn is_running(&self, _pid: u32) -> bool {
        false
    }

    fn terminate(&self, pid: u32) -> Result<(), LocalRuntimeError> {
        Err(LocalRuntimeError::Rejected(format!(
            "process termination adapter unavailable for pid {pid} on this platform"
        )))
    }
}
