//! KubChain consensus error types.

use alloy_primitives::Address;

/// Errors specific to KubChain PoSA consensus.
#[derive(Debug, Clone, thiserror::Error)]
pub enum KubConsensusError {
    /// The block's extra-data is too short (missing vanity or signature).
    #[error("extra-data too short: {length} bytes, minimum {minimum}")]
    ExtraDataTooShort {
        /// Actual length.
        length: usize,
        /// Minimum required length.
        minimum: usize,
    },

    /// The block's extra-data contains validator data at a non-checkpoint block.
    #[error("extra-data has {extra_signers} extra bytes at non-checkpoint block")]
    ExtraSignersAtNonCheckpoint {
        /// Number of unexpected extra bytes.
        extra_signers: usize,
    },

    /// Could not recover signer from the block's extra-data signature.
    #[error("failed to recover block signer from signature")]
    SignerRecoveryFailed,

    /// The recovered signer is not an authorized validator.
    #[error("signer {signer} is not an authorized validator")]
    UnauthorizedSigner {
        /// The recovered signer address.
        signer: Address,
    },

    /// The signer recently signed a block and is not allowed to sign again.
    #[error("signer {signer} recently signed block {recent_block}")]
    RecentlySigned {
        /// The signer address.
        signer: Address,
        /// The recent block number they signed.
        recent_block: u64,
    },

    /// Invalid difficulty value for the block.
    #[error("invalid difficulty {difficulty}, expected {expected}")]
    InvalidDifficulty {
        /// Actual difficulty.
        difficulty: u64,
        /// Expected difficulty.
        expected: u64,
    },

    /// Snapshot not found for the given block.
    #[error("snapshot not found for block {block_number}")]
    SnapshotNotFound {
        /// Block number where snapshot was expected.
        block_number: u64,
    },

    /// System contract call failed.
    #[error("system contract call failed: {reason}")]
    SystemContractCallFailed {
        /// Reason for the failure.
        reason: String,
    },

    /// Invalid validator list in extra-data.
    #[error("invalid validator list in extra-data: {reason}")]
    InvalidValidatorList {
        /// Reason for the validation failure.
        reason: String,
    },
}
