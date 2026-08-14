use crate::experiment::port::WordleDecisionPort;
use crate::experiment::types::{LetterMark, WordleEpisodeProof, WordleStepProof};
use crate::experiment::wordle::observe_guess;

pub fn run_wordle_episode<P: WordleDecisionPort>(
    answer: &str,
    max_steps: u32,
    policy: &P,
) -> WordleEpisodeProof {
    let mut history = Vec::new();
    let mut steps = Vec::new();
    let mut unknowns = Vec::new();
    let mut solved = false;

    if answer.is_empty() || max_steps == 0 {
        unknowns.push("invalid_environment_configuration".into());
    } else {
        for step in 1..=max_steps {
            let Some(guess) = policy.next_guess(&history) else {
                unknowns.push("policy_produced_no_bounded_action".into());
                break;
            };
            let Some(observation) = observe_guess(answer, &guess) else {
                unknowns.push("policy_guess_does_not_match_environment_contract".into());
                break;
            };
            solved = observation
                .marks
                .iter()
                .all(|mark| *mark == LetterMark::Correct);
            history.push(observation.clone());
            steps.push(WordleStepProof {
                step,
                desired_action: guess,
                observation,
                remaining_candidates: policy.remaining_candidates(&history),
            });
            if solved {
                break;
            }
        }
    }
    let status = if solved {
        "solved"
    } else if unknowns.is_empty() {
        "exhausted"
    } else {
        "blocked"
    };
    WordleEpisodeProof {
        schema: "cynic.askesis.wordle.episode_proof.v1",
        status,
        policy: policy.name().into(),
        word_length: answer.chars().count(),
        max_steps,
        solved,
        steps,
        unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment::wordle::CandidateEliminationPolicy;

    #[test]
    fn episode_retains_observations_until_read_back_proves_solution() {
        let policy = CandidateEliminationPolicy::new(vec!["arise".into(), "cigar".into()]);
        let proof = run_wordle_episode("cigar", 6, &policy);
        assert!(proof.solved);
        assert_eq!(proof.status, "solved");
        assert_eq!(proof.steps.last().map(|step| step.desired_action.as_str()), Some("cigar"));
    }

    #[test]
    fn invalid_policy_action_is_blocked_not_successful() {
        let policy = CandidateEliminationPolicy::new(vec!["toolong".into()]);
        let proof = run_wordle_episode("cigar", 6, &policy);
        assert_eq!(proof.status, "blocked");
        assert_eq!(proof.unknowns, vec!["policy_guess_does_not_match_environment_contract"]);
    }
}

