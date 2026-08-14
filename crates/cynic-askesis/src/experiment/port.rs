use crate::experiment::types::WordleObservation;

pub trait WordleDecisionPort {
    fn name(&self) -> &str;
    fn next_guess(&self, history: &[WordleObservation]) -> Option<String>;
    fn remaining_candidates(&self, _history: &[WordleObservation]) -> Option<usize> {
        None
    }
}

