use crate::experiment::port::WordleDecisionPort;
use crate::experiment::types::{LetterMark, WordleObservation};

pub fn observe_guess(answer: &str, guess: &str) -> Option<WordleObservation> {
    if answer.chars().count() != guess.chars().count() || !answer.is_ascii() || !guess.is_ascii() {
        return None;
    }
    let answer = answer.to_ascii_lowercase().into_bytes();
    let guess = guess.to_ascii_lowercase().into_bytes();
    let mut marks = vec![LetterMark::Absent; answer.len()];
    let mut consumed = vec![false; answer.len()];
    for index in 0..answer.len() {
        if answer[index] == guess[index] {
            marks[index] = LetterMark::Correct;
            consumed[index] = true;
        }
    }
    for guess_index in 0..guess.len() {
        if marks[guess_index] == LetterMark::Correct {
            continue;
        }
        if let Some(answer_index) = (0..answer.len())
            .find(|&index| !consumed[index] && answer[index] == guess[guess_index])
        {
            marks[guess_index] = LetterMark::Present;
            consumed[answer_index] = true;
        }
    }
    Some(WordleObservation {
        guess: String::from_utf8(guess).ok()?,
        marks,
    })
}

#[derive(Debug, Clone)]
pub struct CandidateEliminationPolicy {
    candidates: Vec<String>,
}

impl CandidateEliminationPolicy {
    pub fn new(candidates: Vec<String>) -> Self {
        Self { candidates }
    }

    fn candidates_for<'a>(&'a self, history: &[WordleObservation]) -> Vec<&'a str> {
        self.candidates
            .iter()
            .map(String::as_str)
            .filter(|candidate| {
                history.iter().all(|observed| {
                    observe_guess(candidate, &observed.guess)
                        .is_some_and(|candidate_observation| candidate_observation.marks == observed.marks)
                })
            })
            .collect()
    }
}

impl WordleDecisionPort for CandidateEliminationPolicy {
    fn name(&self) -> &str {
        "candidate_elimination.v1"
    }

    fn next_guess(&self, history: &[WordleObservation]) -> Option<String> {
        self.candidates_for(history).first().map(|word| (*word).to_string())
    }

    fn remaining_candidates(&self, history: &[WordleObservation]) -> Option<usize> {
        Some(self.candidates_for(history).len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_letters_are_consumed_once() {
        let observed = observe_guess("cigar", "array").expect("same length");
        assert_eq!(observed.marks, vec![LetterMark::Absent, LetterMark::Present, LetterMark::Absent, LetterMark::Correct, LetterMark::Absent]);
    }
}

