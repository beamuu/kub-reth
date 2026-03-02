//! KubChain consensus types.

use alloy_primitives::{Address, U256};
use alloy_rlp::{RlpDecodable, RlpEncodable};
use serde::{Deserialize, Serialize};

/// The system address used to accumulate transaction fees before distribution.
///
/// Fees are first collected at this address, then transferred to the validator
/// via the StakeManager contract during block finalization.
///
/// Address: `0xffffFFFfFFffffffffffffffFfFFFfffFFFfFFfE`
pub const SYSTEM_ADDRESS: Address = Address::new([
    0xff, 0xff, 0xFF, 0xFf, 0xFF, 0xff, 0xff, 0xff, 0xff, 0xFF, 0xFF, 0xff, 0xFf, 0xFF, 0xFf,
    0xff, 0xff, 0xFF, 0xfF, 0xfE,
]);

/// Difficulty value for in-turn block signers.
pub const DIFF_IN_TURN: u64 = 2;

/// Difficulty value for out-of-turn block signers (fallback/super node).
pub const DIFF_NO_TURN: u64 = 1;

/// Number of blocks between snapshot checkpoints persisted to the database.
pub const CHECKPOINT_INTERVAL: u64 = 1024;

/// Number of recent snapshots to keep in the in-memory LRU cache.
pub const INMEMORY_SNAPSHOTS: usize = 128;

/// Number of recent block signatures to cache for signer recovery.
pub const INMEMORY_SIGNATURES: usize = 4096;

/// Length of the vanity field in header extra-data (bytes).
pub const EXTRA_VANITY: usize = 32;

/// Length of the ECDSA signature in header extra-data (bytes).
pub const EXTRA_SEAL: usize = 65;

/// Length of a single system contract address encoding in extra-data (bytes).
pub const EXTRA_CONTRACT_LEN: usize = 20;

/// Number of system contract addresses encoded in extra-data at span boundaries.
/// (StakeManager + SlashManager + OfficialNode/SuperNode)
pub const EXTRA_NUM_CONTRACTS: usize = 3;

/// Total length of system contracts section in extra-data (bytes).
pub const EXTRA_CONTRACTS_LEN: usize = EXTRA_CONTRACT_LEN * EXTRA_NUM_CONTRACTS;

/// Default epoch length for vote reset and checkpoint (blocks).
pub const DEFAULT_EPOCH: u64 = 30_000;

/// Random delay range for out-of-turn signer to avoid collision (milliseconds).
pub const WIGGLE_TIME_MS: u64 = 100;

/// A KubChain validator with address and voting power (staking weight).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    /// Validator's Ethereum address.
    pub address: Address,
    /// Voting power derived from staking amount.
    pub voting_power: U256,
}

/// Minimal validator representation for RLP encoding in span commitment.
///
/// Used when encoding the validator set for the `commitSpan` system transaction.
#[derive(Debug, Clone, PartialEq, Eq, RlpEncodable, RlpDecodable)]
pub struct MinimalVal {
    /// Validator address.
    pub address: Address,
    /// Voting power.
    pub voting_power: U256,
}

impl From<&Validator> for MinimalVal {
    fn from(v: &Validator) -> Self {
        Self {
            address: v.address,
            voting_power: v.voting_power,
        }
    }
}

/// System contract addresses tracked by the consensus engine.
///
/// These addresses are stored in block header extra-data at span boundaries
/// and used for system transaction routing during block finalization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemContracts {
    /// StakeManager contract - handles staking and reward distribution.
    pub stake_manager: Address,
    /// SlashManager contract - handles slashing penalties for misbehavior.
    pub slash_manager: Address,
    /// OfficialNode contract - official node pool (pre-Basel).
    /// SuperNode contract - super node management (Basel onwards).
    pub super_node: Address,
}

/// Configuration for a KubChain PoSA span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpanConfig {
    /// Number of blocks in each span.
    pub span: u64,
    /// Block time in seconds.
    pub period: u64,
    /// Epoch length for vote reset.
    pub epoch: u64,
}

impl Default for SpanConfig {
    fn default() -> Self {
        Self {
            span: 0,
            period: 15,
            epoch: DEFAULT_EPOCH,
        }
    }
}

/// Returns true if the given block number is the first block of a span.
///
/// `block_number % span == 0`
pub fn is_span_first_block(span: u64, block_number: u64) -> bool {
    if span == 0 {
        return false;
    }
    block_number % span == 0
}

/// Returns true if the given block number is the span commitment block.
///
/// Validator set for the next span is committed at `span/2 + 1` within each span.
/// `(block_number % span) == (span / 2 + 1)`
pub fn is_span_commitment_block(span: u64, block_number: u64) -> bool {
    if span == 0 {
        return false;
    }
    (block_number % span) == (span / 2 + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_first_block() {
        let span = 200;
        assert!(is_span_first_block(span, 0));
        assert!(is_span_first_block(span, 200));
        assert!(is_span_first_block(span, 400));
        assert!(!is_span_first_block(span, 1));
        assert!(!is_span_first_block(span, 199));
        assert!(!is_span_first_block(span, 201));
    }

    #[test]
    fn test_span_commitment_block() {
        let span = 200;
        // span/2 + 1 = 101
        assert!(is_span_commitment_block(span, 101));
        assert!(is_span_commitment_block(span, 301));
        assert!(!is_span_commitment_block(span, 100));
        assert!(!is_span_commitment_block(span, 102));
        assert!(!is_span_commitment_block(span, 0));
    }

    #[test]
    fn test_span_zero_safety() {
        assert!(!is_span_first_block(0, 0));
        assert!(!is_span_commitment_block(0, 0));
    }

    #[test]
    fn test_system_address() {
        assert_eq!(
            format!("{:?}", SYSTEM_ADDRESS),
            "0xfffffffffffffffffffffffffffffffffffffffe"
        );
    }
}
