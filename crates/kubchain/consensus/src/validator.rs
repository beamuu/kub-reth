//! Weighted random validator selection for KubChain PoSA.
//!
//! Implements the deterministic validator selection algorithm from
//! `bkc/consensus/clique/clique.go:1390-1441`.
//!
//! The algorithm:
//! 1. Get eligible validators with their voting power (staking weight)
//! 2. Use a seed derived from a block hash (5 blocks prior) for randomness
//! 3. Perform weighted random selection via binary search on cumulative weights
//! 4. Repeat until all validator positions are filled

use alloy_primitives::{B256, U256};
use kubchain_primitives::Validator;

/// Number of blocks to look back for the random seed.
///
/// The seed is derived from the hash of the block 5 positions before the
/// target block, providing deterministic randomness.
pub const SEED_LOOKBACK: u64 = 5;

/// Selects a validator set for the next span using weighted random selection.
///
/// # Arguments
/// * `eligible` - Validators with their voting power
/// * `seed` - Random seed (block hash from 5 blocks prior)
/// * `count` - Number of validators to select
///
/// # Returns
/// Ordered vector of selected validator addresses.
///
/// # Algorithm
/// For each position, a random number is generated in [0, total_weight) using
/// the seed. Binary search on the cumulative weight array determines which
/// validator is selected. The selected validator is then removed from the
/// candidate pool.
pub fn select_validators(eligible: &[Validator], seed: B256, count: usize) -> Vec<Validator> {
    if eligible.is_empty() || count == 0 {
        return Vec::new();
    }

    let mut candidates: Vec<Validator> = eligible.to_vec();
    let mut selected = Vec::with_capacity(count);
    let mut current_seed = seed;

    for _ in 0..count {
        if candidates.is_empty() {
            break;
        }

        // Build cumulative weight array
        let cumulative_weights = build_cumulative_weights(&candidates);
        let total_weight = cumulative_weights.last().copied().unwrap_or(U256::ZERO);

        if total_weight.is_zero() {
            break;
        }

        // Generate random number in [0, total_weight)
        let random = random_range(current_seed, total_weight);

        // Binary search for the validator
        let idx = binary_search(&cumulative_weights, random);

        // Select the validator
        let validator = candidates.remove(idx);
        selected.push(validator);

        // Update seed for next iteration (hash the current seed)
        current_seed = alloy_primitives::keccak256(current_seed);
    }

    selected
}

/// Builds a cumulative weight array from validators' voting power.
///
/// `cumulative[i] = sum(power[0..=i])`
fn build_cumulative_weights(validators: &[Validator]) -> Vec<U256> {
    let mut cumulative = Vec::with_capacity(validators.len());
    let mut sum = U256::ZERO;

    for v in validators {
        sum = sum.wrapping_add(v.voting_power);
        cumulative.push(sum);
    }

    cumulative
}

/// Generates a random U256 in the range [0, max) from a seed.
///
/// Uses the seed as a source of randomness and takes modulo max.
fn random_range(seed: B256, max: U256) -> U256 {
    let seed_uint = U256::from_be_bytes(seed.0);
    seed_uint % max
}

/// Binary search for the first index where cumulative_weights[i] > target.
///
/// This corresponds to the weighted random selection: the validator whose
/// cumulative weight range includes the random target is selected.
fn binary_search(cumulative_weights: &[U256], target: U256) -> usize {
    let mut lo = 0usize;
    let mut hi = cumulative_weights.len();

    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if cumulative_weights[mid] <= target {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }

    // Clamp to valid range
    lo.min(cumulative_weights.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::Address;

    fn test_validators() -> Vec<Validator> {
        vec![
            Validator {
                address: Address::repeat_byte(0x01),
                voting_power: U256::from(100),
            },
            Validator {
                address: Address::repeat_byte(0x02),
                voting_power: U256::from(200),
            },
            Validator {
                address: Address::repeat_byte(0x03),
                voting_power: U256::from(300),
            },
        ]
    }

    #[test]
    fn test_cumulative_weights() {
        let validators = test_validators();
        let weights = build_cumulative_weights(&validators);

        assert_eq!(weights.len(), 3);
        assert_eq!(weights[0], U256::from(100));
        assert_eq!(weights[1], U256::from(300));
        assert_eq!(weights[2], U256::from(600));
    }

    #[test]
    fn test_binary_search() {
        let weights = vec![U256::from(100), U256::from(300), U256::from(600)];

        // Target 0-99 → index 0 (validator 1)
        assert_eq!(binary_search(&weights, U256::from(0)), 0);
        assert_eq!(binary_search(&weights, U256::from(99)), 0);

        // Target 100-299 → index 1 (validator 2)
        assert_eq!(binary_search(&weights, U256::from(100)), 1);
        assert_eq!(binary_search(&weights, U256::from(299)), 1);

        // Target 300-599 → index 2 (validator 3)
        assert_eq!(binary_search(&weights, U256::from(300)), 2);
        assert_eq!(binary_search(&weights, U256::from(599)), 2);
    }

    #[test]
    fn test_select_validators() {
        let validators = test_validators();
        let seed = B256::repeat_byte(0x42);

        let selected = select_validators(&validators, seed, 2);
        assert_eq!(selected.len(), 2);

        // All selected should be from the original set
        for v in &selected {
            assert!(validators.iter().any(|orig| orig.address == v.address));
        }

        // No duplicates
        let addresses: Vec<_> = selected.iter().map(|v| v.address).collect();
        let unique: std::collections::HashSet<_> = addresses.iter().collect();
        assert_eq!(addresses.len(), unique.len());
    }

    #[test]
    fn test_select_more_than_available() {
        let validators = test_validators();
        let seed = B256::repeat_byte(0x42);

        let selected = select_validators(&validators, seed, 10);
        assert_eq!(selected.len(), 3); // Can't select more than available
    }

    #[test]
    fn test_deterministic_selection() {
        let validators = test_validators();
        let seed = B256::repeat_byte(0x42);

        let selected1 = select_validators(&validators, seed, 3);
        let selected2 = select_validators(&validators, seed, 3);

        // Same seed should produce same selection
        for (a, b) in selected1.iter().zip(selected2.iter()) {
            assert_eq!(a.address, b.address);
        }
    }

    #[test]
    fn test_empty_validators() {
        let selected = select_validators(&[], B256::ZERO, 5);
        assert!(selected.is_empty());
    }
}
