use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct GovernanceContract {
    /// Map of proposal_id -> proposal_data
    proposals: UnorderedMap<String, Proposal>,
    /// Map of proposal_id -> votes
    votes: UnorderedMap<String, UnorderedMap<AccountId, bool>>,
    /// Contract owner
    owner: AccountId,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct Proposal {
    id: String,
    title: String,
    description: String,
    creator: AccountId,
    voting_period: u64,
    created_at: u64,
    executed: bool,
}

#[near_bindgen]
impl GovernanceContract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        GovernanceContract {
            proposals: UnorderedMap::new(b"p"),
            votes: UnorderedMap::new(b"v"),
            owner,
        }
    }

    pub fn create_proposal(
        &mut self,
        id: String,
        title: String,
        description: String,
    ) -> Proposal {
        let proposal = Proposal {
            id: id.clone(),
            title,
            description,
            creator: env::signer_account_id(),
            voting_period: 72 * 3600, // 72 hours
            created_at: env::block_timestamp(),
            executed: false,
        };

        self.proposals.insert(id.clone(), proposal.clone());
        self.votes.insert(id, UnorderedMap::new(b"v"));

        proposal
    }

    pub fn vote(&mut self, proposal_id: String, choice: bool) -> bool {
        let voter = env::signer_account_id();

        if let Some(mut votes) = self.votes.get(&proposal_id) {
            votes.insert(voter, choice);
            self.votes.insert(proposal_id, votes);
            true
        } else {
            false
        }
    }

    pub fn get_proposal(&self, id: String) -> Option<Proposal> {
        self.proposals.get(&id)
    }

    pub fn execute_proposal(&mut self, id: String) -> bool {
        if let Some(mut proposal) = self.proposals.get(&id) {
            proposal.executed = true;
            self.proposals.insert(id, proposal);
            true
        } else {
            false
        }
    }
}
