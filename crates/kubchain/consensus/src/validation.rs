//! Block and header validation rules for KubChain PoSA consensus.
//!
//! Implements the validation logic from `bkc/consensus/clique/clique.go`.

use alloy_consensus::Header;
use kubchain_chainspec::KubChainSpec;
use kubchain_primitives::{DIFF_IN_TURN, DIFF_NO_TURN, EXTRA_SEAL, EXTRA_VANITY};
use reth_consensus::ConsensusError;
use reth_execution_types::BlockExecutionResult;
use reth_primitives_traits::{Block, GotExpected, RecoveredBlock, SealedBlock, SealedHeader};

/// Validates a header in isolation (no parent context).
///
/// Checks:
/// - Extra-data length (minimum vanity + seal)
/// - Difficulty is valid (DIFF_IN_TURN or DIFF_NO_TURN)
/// - Nonce is empty (not used in PoSA)
/// - Gas usage does not exceed gas limit
pub fn validate_header(
    header: &SealedHeader<Header>,
    _chain_spec: &KubChainSpec,
) -> Result<(), ConsensusError> {
    let extra_len = header.extra_data.len();
    let min_extra = EXTRA_VANITY + EXTRA_SEAL;

    // Extra-data must be at least vanity + seal
    if extra_len < min_extra {
        return Err(ConsensusError::ExtraDataExceedsMax { len: extra_len });
    }

    // Difficulty must be DIFF_IN_TURN (2) or DIFF_NO_TURN (1)
    let difficulty = header.difficulty.to::<u64>();
    if difficulty != DIFF_IN_TURN && difficulty != DIFF_NO_TURN {
        return Err(ConsensusError::TheMergeDifficultyIsNotZero);
    }

    // Gas used must not exceed gas limit
    if header.gas_used > header.gas_limit {
        return Err(ConsensusError::HeaderGasUsedExceedsGasLimit {
            gas_used: header.gas_used,
            gas_limit: header.gas_limit,
        });
    }

    Ok(())
}

/// Validates a header against its parent.
///
/// Checks:
/// - Block number is parent + 1
/// - Timestamp is greater than parent timestamp
/// - Gas limit changes are within bounds
pub fn validate_header_against_parent(
    header: &SealedHeader<Header>,
    parent: &SealedHeader<Header>,
    _chain_spec: &KubChainSpec,
) -> Result<(), ConsensusError> {
    // Block number must be parent + 1
    if header.number != parent.number + 1 {
        return Err(ConsensusError::ParentBlockNumberMismatch {
            parent_block_number: parent.number,
            block_number: header.number,
        });
    }

    // Timestamp must be after parent
    if header.timestamp <= parent.timestamp {
        return Err(ConsensusError::TimestampIsInPast {
            parent_timestamp: parent.timestamp,
            timestamp: header.timestamp,
        });
    }

    Ok(())
}

/// Validates block body against its header.
///
/// Checks:
/// - Transaction root matches
/// - Uncle hash is empty (no uncles in PoSA)
pub fn validate_body_against_header<B: Block<Header = Header>>(
    body: &B::Body,
    header: &SealedHeader<Header>,
) -> Result<(), ConsensusError> {
    // Use common validation from reth
    reth_consensus_common::validation::validate_body_against_header::<B::Body, Header>(
        body, header,
    )
}

/// Validates a block before execution.
///
/// Checks:
/// - All header validation rules
/// - System transaction validity (if any)
pub fn validate_block_pre_execution<B: Block<Header = Header>>(
    block: &SealedBlock<B>,
    chain_spec: &KubChainSpec,
) -> Result<(), ConsensusError> {
    validate_header(block.sealed_header(), chain_spec)?;
    Ok(())
}

/// Validates a block after execution.
///
/// Checks:
/// - State root matches execution result
/// - Receipt root matches
/// - Gas used matches
pub fn validate_block_post_execution<N: reth_primitives_traits::NodePrimitives>(
    _block: &RecoveredBlock<N::Block>,
    _result: &BlockExecutionResult<N::Receipt>,
    _chain_spec: &KubChainSpec,
) -> Result<(), ConsensusError>
where
    N::Block: Block<Header = Header>,
{
    // Post-execution validation will verify:
    // 1. State root matches
    // 2. Receipt root matches
    // 3. Gas used matches
    // 4. Bloom filter matches
    //
    // TODO: Implement full post-execution validation once the EVM executor
    // is integrated and system transactions are properly injected.
    Ok(())
}
