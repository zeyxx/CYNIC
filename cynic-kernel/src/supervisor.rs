use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

/// The Ring 1 Supervisor is responsible for spawning, monitoring, and 
/// automatically restarting Ring 2 Daemons (Crash-Only Software philosophy).
pub struct DaemonSupervisor {
    name: String,
    executable_path: String,
    args: Vec<String>,
    max_restarts: u32,
}

impl DaemonSupervisor {
    pub fn new(name: &str, executable_path: &str, args: Vec<&str>, max_restarts: u32) -> Self {
        Self {
            name: name.to_string(),
            executable_path: executable_path.to_string(),
            args: args.into_iter().map(String::from).collect(),
            max_restarts,
        }
    }

    /// Spawns the daemon and monitors its lifecycle. If it crashes, it restarts it.
    pub async fn supervise(&self) {
        let mut restarts = 0;

        loop {
            println!("[Supervisor] Spawning Daemon '{}'...", self.name);

            // Configure the child process
            let child = Command::new(&self.executable_path)
                .args(&self.args)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn();

            match child {
                Ok(mut process) => {
                    // Wait for the process to exit
                    let status = process.wait().await.unwrap();

                    println!("[Supervisor] Daemon '{}' exited with status: {}", self.name, status);

                    if status.success() {
                        println!("[Supervisor] Daemon '{}' completed successfully. Stopping supervision.", self.name);
                        break;
                    }
                }
                Err(e) => {
                    println!("[Supervisor] Failed to spawn Daemon '{}': {}", self.name, e);
                }
            }

            // Crash isolated, handle restart
            restarts += 1;
            if restarts > self.max_restarts {
                println!("[Supervisor] FATAL: Daemon '{}' exceeded max restarts ({}). Halting supervision.", self.name, self.max_restarts);
                // Here we would ideally emit an Event to the EventBus informing the OS
                break;
            }

            println!("[Supervisor] Restarting Daemon '{}' in 2 seconds... (Retry {}/{})", self.name, restarts, self.max_restarts);
            sleep(Duration::from_secs(2)).await;
        }
    }
}
