use crate::error::AskesisError;
use crate::eval::types::GenerationPolicy;

pub fn validate_policy(policy: &GenerationPolicy) -> crate::Result<()> {
    if policy.name.trim().is_empty() {
        return Err(AskesisError::EvalPolicy("policy name is empty".into()));
    }
    if policy.max_tokens == 0 {
        return Err(AskesisError::EvalPolicy(format!(
            "{}: max_tokens must be greater than zero",
            policy.name
        )));
    }
    for (name, value) in [
        ("temperature", policy.temperature),
        ("top_p", policy.top_p),
        ("min_p", policy.min_p),
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(AskesisError::EvalPolicy(format!(
                "{}: {name} must be finite and non-negative",
                policy.name
            )));
        }
    }
    if policy.top_p > 1.0 || policy.min_p > 1.0 {
        return Err(AskesisError::EvalPolicy(format!(
            "{}: top_p and min_p must not exceed 1",
            policy.name
        )));
    }
    if policy.repeat_penalty <= 0.0 || policy.timeout_secs == 0 {
        return Err(AskesisError::EvalPolicy(format!(
            "{}: repeat_penalty and timeout_secs must be positive",
            policy.name
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> GenerationPolicy {
        GenerationPolicy {
            name: "coding".into(),
            max_tokens: 256,
            temperature: 0.0,
            top_p: 0.95,
            top_k: 40,
            min_p: 0.05,
            repeat_penalty: 1.0,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            seed: 42,
            enable_thinking: false,
            json_mode: false,
            timeout_secs: 120,
            stop: vec![],
        }
    }

    #[test]
    fn deterministic_coding_policy_is_valid() {
        assert!(validate_policy(&policy()).is_ok());
    }

    #[test]
    fn zero_token_budget_is_rejected() {
        let mut candidate = policy();
        candidate.max_tokens = 0;
        assert!(validate_policy(&candidate).is_err());
    }
}
