use std::{
    fs::File,
    process::{Command, Stdio},
};

use cynic_protocol::RuntimeSeedRequest;

use super::filesystem::regular_file;

pub trait RuntimeSeedSignatureVerifier: Send + Sync {
    fn verify(&self, request: &RuntimeSeedRequest) -> Result<(), String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SshRuntimeSeedSignatureVerifier;

impl RuntimeSeedSignatureVerifier for SshRuntimeSeedSignatureVerifier {
    fn verify(&self, request: &RuntimeSeedRequest) -> Result<(), String> {
        regular_file(&request.basket_path, "basket")?;
        regular_file(&request.detached_signature_path, "signature")?;
        regular_file(&request.allowed_signers_path, "allowed_signers")?;
        let payload = File::open(&request.basket_path)
            .map_err(|error| format!("runtime_seed_basket_open_failed:{error}"))?;
        let mut child = Command::new("ssh-keygen")
            .args([
                "-Y",
                "verify",
                "-f",
                &request.allowed_signers_path,
                "-I",
                &request.signer_identity,
                "-n",
                &request.signature_namespace,
                "-s",
                &request.detached_signature_path,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("runtime_seed_signature_spawn_failed:{error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "runtime_seed_signature_stdin_unavailable".to_string())?;
        let mut payload = payload;
        std::io::copy(&mut payload, &mut stdin)
            .map_err(|error| format!("runtime_seed_signature_input_failed:{error}"))?;
        drop(stdin);
        let output = child
            .wait_with_output()
            .map_err(|error| format!("runtime_seed_signature_wait_failed:{error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "runtime_seed_signature_invalid:{}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
}
