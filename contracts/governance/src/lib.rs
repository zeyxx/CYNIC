/// NEAR Governance Contract for CYNIC
///
/// This contract stores governance proposals with CYNIC verdicts and enables
/// community voting. Proposals include:
/// - Title and description
/// - CYNIC verdict (HOWL/WAG/GROWL/BARK)
/// - CYNIC confidence score (0-100)
/// - Vote tallies (for/against/abstain)
/// - Status tracking (Open/Voted/Approved/Rejected/Executed)

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use serde::{Deserialize, Serialize};

/// Proposal status states
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum ProposalStatus {
    Open,
    Voted,
    Approved,
    Rejected,
    Executed,
}

/// A governance proposal with CYNIC verdict attached
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Proposal {
    /// Unique proposal identifier
    pub proposal_id: String,
    /// Proposal title
    pub title: String,
    /// Proposal description
    pub description: String,
    /// CYNIC's verdict: HOWL/WAG/GROWL/BARK
    pub cynic_verdict: String,
    /// CYNIC's confidence score (0-100)
    pub cynic_q_score: u32,
    /// Vote count: supporting
    pub votes_for: u64,
    /// Vote count: opposing
    pub votes_against: u64,
    /// Vote count: abstaining
    pub votes_abstain: u64,
    /// Current proposal status
    pub status: ProposalStatus,
    /// Timestamp when proposal was created (seconds)
    pub created_at: u64,
    /// Timestamp when voting closes (seconds)
    pub expires_at: u64,
}

/// Governance contract
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Governance {
    /// Storage for all proposals
    proposals: UnorderedMap<String, Proposal>,
    /// Contract owner
    owner: AccountId,
}

#[near_bindgen]
impl Governance {
    /// Initialize a new governance contract
    ///
    /// # Arguments
    /// * `owner` - Account ID of contract owner
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            proposals: UnorderedMap::new(b"p".to_vec()),
            owner,
        }
    }

    /// Create a new governance proposal with CYNIC verdict attached
    ///
    /// # Arguments
    /// * `proposal_id` - Unique proposal identifier
    /// * `title` - Proposal title
    /// * `description` - Proposal description
    /// * `cynic_verdict` - CYNIC's verdict (HOWL/WAG/GROWL/BARK)
    /// * `cynic_q_score` - CYNIC's confidence score (0-100)
    /// * `expires_at` - Unix timestamp when voting closes
    ///
    /// # Returns
    /// The created proposal
    pub fn create_proposal(
        &mut self,
        proposal_id: String,
        title: String,
        description: String,
        cynic_verdict: String,
        cynic_q_score: u32,
        expires_at: u64,
    ) -> Proposal {
        // Only owner can create proposals
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can create proposals"
        );

        // Validate verdict
        assert!(
            matches!(
                cynic_verdict.as_str(),
                "HOWL" | "WAG" | "GROWL" | "BARK"
            ),
            "Invalid verdict"
        );

        // Validate score
        assert!(
            cynic_q_score <= 100,
            "Q-score must be between 0 and 100"
        );

        let proposal = Proposal {
            proposal_id: proposal_id.clone(),
            title,
            description,
            cynic_verdict,
            cynic_q_score,
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            status: ProposalStatus::Open,
            created_at: env::block_timestamp() / 1_000_000,
            expires_at,
        };

        self.proposals.insert(&proposal_id, &proposal);

        env::log_str(&format!(
            "Proposal created: proposal_id={}, verdict={}",
            proposal_id, proposal.cynic_verdict
        ));

        proposal
    }

    /// Record a vote on a proposal
    ///
    /// # Arguments
    /// * `proposal_id` - Proposal to vote on
    /// * `vote` - Vote type: "for", "against", or "abstain"
    ///
    /// # Returns
    /// True if vote was recorded
    pub fn vote(&mut self, proposal_id: String, vote: String) -> bool {
        let mut proposal = self
            .proposals
            .get(&proposal_id)
            .expect("Proposal not found");

        // Validate vote type
        assert!(
            matches!(vote.as_str(), "for" | "against" | "abstain"),
            "Invalid vote type"
        );

        // Record vote
        match vote.as_str() {
            "for" => proposal.votes_for += 1,
            "against" => proposal.votes_against += 1,
            "abstain" => proposal.votes_abstain += 1,
            _ => panic!("Invalid vote type"),
        }

        // Update status if voting is active
        proposal.status = ProposalStatus::Voted;

        self.proposals.insert(&proposal_id, &proposal);

        env::log_str(&format!(
            "Vote recorded: proposal_id={}, vote={}",
            proposal_id, vote
        ));

        true
    }

    /// Execute an approved proposal
    ///
    /// # Arguments
    /// * `proposal_id` - Proposal to execute
    ///
    /// # Returns
    /// True if proposal was executed
    pub fn execute_proposal(&mut self, proposal_id: String) -> bool {
        let mut proposal = self
            .proposals
            .get(&proposal_id)
            .expect("Proposal not found");

        // Calculate approval percentage
        let total_votes = proposal.votes_for + proposal.votes_against;
        assert!(total_votes > 0, "No votes recorded");

        let approval_pct = (proposal.votes_for * 100) / total_votes;

        // Require 50% approval to execute
        assert!(
            approval_pct >= 50,
            "Proposal does not have majority approval"
        );

        proposal.status = ProposalStatus::Executed;
        self.proposals.insert(&proposal_id, &proposal);

        env::log_str(&format!(
            "Proposal executed: proposal_id={}, approval={}%",
            proposal_id, approval_pct
        ));

        true
    }

    /// Get a proposal by ID
    ///
    /// # Arguments
    /// * `proposal_id` - Proposal identifier
    ///
    /// # Returns
    /// The proposal if found, None otherwise
    pub fn get_proposal(&self, proposal_id: String) -> Option<Proposal> {
        self.proposals.get(&proposal_id)
    }

    /// Get total number of proposals
    pub fn get_proposals_count(&self) -> u64 {
        self.proposals.len()
    }

    /// Get contract owner
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proposal() {
        let owner = AccountId::new_unchecked("owner.testnet".to_string());
        let mut governance = Governance::new(owner.clone());

        let proposal = governance.create_proposal(
            "prop_1".to_string(),
            "Test Proposal".to_string(),
            "Test description".to_string(),
            "WAG".to_string(),
            75,
            1740000000,
        );

        assert_eq!(proposal.proposal_id, "prop_1");
        assert_eq!(proposal.cynic_verdict, "WAG");
        assert_eq!(proposal.cynic_q_score, 75);
        assert_eq!(proposal.votes_for, 0);
    }

    #[test]
    fn test_vote() {
        let owner = AccountId::new_unchecked("owner.testnet".to_string());
        let mut governance = Governance::new(owner.clone());

        governance.create_proposal(
            "prop_1".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            "WAG".to_string(),
            75,
            1740000000,
        );

        governance.vote("prop_1".to_string(), "for".to_string());
        governance.vote("prop_1".to_string(), "against".to_string());

        let proposal = governance.get_proposal("prop_1".to_string()).unwrap();
        assert_eq!(proposal.votes_for, 1);
        assert_eq!(proposal.votes_against, 1);
    }

    #[test]
    fn test_execute_proposal() {
        let owner = AccountId::new_unchecked("owner.testnet".to_string());
        let mut governance = Governance::new(owner.clone());

        governance.create_proposal(
            "prop_1".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            "WAG".to_string(),
            75,
            1740000000,
        );

        // Vote in favor
        governance.vote("prop_1".to_string(), "for".to_string());
        governance.vote("prop_1".to_string(), "for".to_string());

        // Execute
        governance.execute_proposal("prop_1".to_string());

        let proposal = governance.get_proposal("prop_1".to_string()).unwrap();
        assert!(matches!(proposal.status, ProposalStatus::Executed));
    }

    #[test]
    #[should_panic]
    fn test_invalid_verdict() {
        let owner = AccountId::new_unchecked("owner.testnet".to_string());
        let mut governance = Governance::new(owner.clone());

        governance.create_proposal(
            "prop_1".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            "INVALID".to_string(),
            75,
            1740000000,
        );
    }
}
