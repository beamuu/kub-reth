//! KubChain validator snapshot management.
//!
//! Snapshots track the validator set and system contract addresses at specific
//! block heights. They are persisted to the database at checkpoint intervals
//! (every 1024 blocks) and cached in memory for fast access.
//!
//! Port of: `bkc/consensus/clique/snapshot.go`

use alloy_primitives::{Address, B256};
use kubchain_primitives::SystemContracts;
use std::collections::{HashMap, HashSet};

/// A validator set snapshot at a specific block.
///
/// Captures the full consensus state needed to validate subsequent blocks.
#[derive(Debug, Clone)]
pub struct KubSnapshot {
    /// Block number this snapshot was taken at.
    pub number: u64,
    /// Block hash this snapshot corresponds to.
    pub hash: B256,
    /// Set of authorized signers (pre-PoSA).
    pub signers: HashSet<Address>,
    /// Ordered list of validators (PoSA).
    pub validators: Vec<Address>,
    /// Recent signer mapping: block_number → signer_address.
    ///
    /// Used to prevent a signer from signing too frequently.
    pub recents: HashMap<u64, Address>,
    /// Current system contract addresses.
    pub system_contracts: SystemContracts,
}

impl KubSnapshot {
    /// Creates a new snapshot at the given block.
    pub fn new(
        number: u64,
        hash: B256,
        validators: Vec<Address>,
        system_contracts: SystemContracts,
    ) -> Self {
        let signers: HashSet<Address> = validators.iter().copied().collect();
        Self {
            number,
            hash,
            signers,
            validators,
            recents: HashMap::new(),
            system_contracts,
        }
    }

    /// Returns true if the given address is an authorized validator.
    pub fn is_authorized(&self, addr: &Address) -> bool {
        self.signers.contains(addr)
    }

    /// Returns the number of authorized signers.
    pub fn num_signers(&self) -> usize {
        self.signers.len()
    }

    /// Returns the in-turn signer for a given block number.
    ///
    /// The in-turn signer is determined by `block_number % num_validators`.
    /// If there are no validators, returns `None`.
    pub fn in_turn_signer(&self, block_number: u64) -> Option<&Address> {
        if self.validators.is_empty() {
            return None;
        }
        let idx = (block_number as usize) % self.validators.len();
        self.validators.get(idx)
    }

    /// Returns true if the given signer has recently signed a block
    /// within the recency window (num_signers / 2 + 1).
    ///
    /// This prevents validators from signing consecutive blocks and
    /// ensures fair block production distribution.
    pub fn is_recently_signed(&self, signer: &Address, block_number: u64) -> bool {
        if self.signers.is_empty() {
            return false;
        }

        let limit = self.signers.len() / 2 + 1;
        let start = if block_number >= limit as u64 {
            block_number - limit as u64 + 1
        } else {
            0
        };

        for num in start..block_number {
            if let Some(recent_signer) = self.recents.get(&num) {
                if recent_signer == signer {
                    return true;
                }
            }
        }
        false
    }

    /// Applies a new block to this snapshot, updating the recents map.
    ///
    /// Returns a new snapshot for the given block.
    pub fn apply(&self, number: u64, hash: B256, signer: Address) -> Self {
        let mut new_snap = self.clone();
        new_snap.number = number;
        new_snap.hash = hash;

        // Add the signer to recents
        new_snap.recents.insert(number, signer);

        // Prune old recents beyond the window
        let limit = new_snap.signers.len() / 2 + 1;
        if number >= limit as u64 {
            let prune_before = number - limit as u64;
            new_snap.recents.retain(|&k, _| k > prune_before);
        }

        new_snap
    }

    /// Updates the validator set (used at span boundaries).
    pub fn update_validators(
        &mut self,
        validators: Vec<Address>,
        system_contracts: SystemContracts,
    ) {
        self.signers = validators.iter().copied().collect();
        self.validators = validators;
        self.system_contracts = system_contracts;
        // Clear recents when validator set changes
        self.recents.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validators() -> Vec<Address> {
        vec![
            Address::repeat_byte(0x01),
            Address::repeat_byte(0x02),
            Address::repeat_byte(0x03),
        ]
    }

    #[test]
    fn test_new_snapshot() {
        let validators = test_validators();
        let snap = KubSnapshot::new(
            100,
            B256::ZERO,
            validators.clone(),
            SystemContracts::default(),
        );

        assert_eq!(snap.number, 100);
        assert_eq!(snap.validators.len(), 3);
        assert_eq!(snap.signers.len(), 3);
        assert!(snap.is_authorized(&validators[0]));
        assert!(!snap.is_authorized(&Address::repeat_byte(0xFF)));
    }

    #[test]
    fn test_in_turn_signer() {
        let validators = test_validators();
        let snap = KubSnapshot::new(0, B256::ZERO, validators.clone(), SystemContracts::default());

        assert_eq!(snap.in_turn_signer(0), Some(&validators[0]));
        assert_eq!(snap.in_turn_signer(1), Some(&validators[1]));
        assert_eq!(snap.in_turn_signer(2), Some(&validators[2]));
        assert_eq!(snap.in_turn_signer(3), Some(&validators[0]));
    }

    #[test]
    fn test_recently_signed() {
        let validators = test_validators();
        let mut snap =
            KubSnapshot::new(0, B256::ZERO, validators.clone(), SystemContracts::default());
        snap.recents.insert(10, validators[0]);

        // limit = 3/2 + 1 = 2
        // For block 11, check window [10, 11) → validator[0] at 10 is in window
        assert!(snap.is_recently_signed(&validators[0], 11));

        // For block 13, check window [12, 13) → validator[0] at 10 is out of window
        assert!(!snap.is_recently_signed(&validators[0], 13));
    }

    #[test]
    fn test_apply_prunes_recents() {
        let validators = test_validators();
        let snap =
            KubSnapshot::new(0, B256::ZERO, validators.clone(), SystemContracts::default());

        let snap = snap.apply(10, B256::repeat_byte(0x10), validators[0]);
        assert!(snap.recents.contains_key(&10));

        let snap = snap.apply(11, B256::repeat_byte(0x11), validators[1]);
        assert!(snap.recents.contains_key(&11));
        // block 10 should be pruned (limit = 2, prune_before = 11 - 2 = 9, 10 > 9 so kept)
        assert!(snap.recents.contains_key(&10));

        let snap = snap.apply(12, B256::repeat_byte(0x12), validators[2]);
        // block 10 should be pruned (limit = 2, prune_before = 12 - 2 = 10, 10 is NOT > 10)
        assert!(!snap.recents.contains_key(&10));
        assert!(snap.recents.contains_key(&11));
    }
}
