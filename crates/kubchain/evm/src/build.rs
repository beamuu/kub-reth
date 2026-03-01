//! KubChain block assembler.
//!
//! Assembles a complete block from execution results. KubChain blocks differ
//! from Ethereum blocks in several ways:
//!
//! - **Difficulty**: Non-zero (2 for in-turn, 1 for out-of-turn)
//! - **Extra-data**: Contains validator set + system contracts at span boundaries
//! - **No withdrawals**: No Shanghai withdrawal support
//! - **No blobs**: No EIP-4844 blob gas fields
//! - **No beacon root**: No parent_beacon_block_root

use crate::execute::KubBlockExecutionCtx;
use alloc::sync::Arc;
use alloy_consensus::{
    proofs, Block, BlockBody, Header, TxReceipt, EMPTY_OMMER_ROOT_HASH,
};
use alloy_evm::block::BlockExecutorFactory;
use kubchain_chainspec::KubChainSpec;
use reth_ethereum_primitives::{Receipt, TransactionSigned};
use reth_evm::execute::{BlockAssembler, BlockAssemblerInput, BlockExecutionError};
use reth_execution_types::BlockExecutionResult;
use reth_primitives_traits::logs_bloom;

extern crate alloc;

/// Block assembler for KubChain.
///
/// Creates the final block from execution results, applying KubChain-specific
/// header field rules.
#[derive(Debug, Clone)]
pub struct KubBlockAssembler {
    /// Chain specification.
    pub chain_spec: Arc<KubChainSpec>,
}

impl KubBlockAssembler {
    /// Creates a new [`KubBlockAssembler`].
    pub fn new(chain_spec: Arc<KubChainSpec>) -> Self {
        Self { chain_spec }
    }
}

impl<F> BlockAssembler<F> for KubBlockAssembler
where
    F: for<'a> BlockExecutorFactory<
        ExecutionCtx<'a> = KubBlockExecutionCtx,
        Transaction = TransactionSigned,
        Receipt = Receipt,
    >,
{
    type Block = Block<TransactionSigned>;

    fn assemble_block(
        &self,
        input: BlockAssemblerInput<'_, '_, F>,
    ) -> Result<Block<TransactionSigned>, BlockExecutionError> {
        let BlockAssemblerInput {
            evm_env,
            execution_ctx: ctx,
            transactions,
            output: BlockExecutionResult { receipts, gas_used, .. },
            state_root,
            ..
        } = input;

        let timestamp: u64 = evm_env.block_env.timestamp.saturating_to();

        // Calculate Merkle roots.
        let transactions_root = proofs::calculate_transaction_root(&transactions);
        let receipts_root = Receipt::calculate_receipt_root_no_memo(receipts);
        let logs_bloom = logs_bloom(receipts.iter().flat_map(|r| TxReceipt::logs(r)));

        let header = Header {
            parent_hash: ctx.parent_hash,
            ommers_hash: EMPTY_OMMER_ROOT_HASH,
            beneficiary: evm_env.block_env.beneficiary,
            state_root,
            transactions_root,
            receipts_root,
            logs_bloom,
            timestamp,
            number: evm_env.block_env.number.saturating_to(),
            gas_limit: evm_env.block_env.gas_limit,
            gas_used: *gas_used,
            // KubChain: difficulty is non-zero (set by consensus, carried through env)
            difficulty: evm_env.block_env.difficulty,
            // KubChain: extra_data will be set by the consensus layer before sealing.
            // During block building, we leave it as default; the consensus
            // layer fills in vanity + validators + contracts + signature.
            extra_data: Default::default(),
            // KubChain: No mix_hash / prevrandao (not PoS merge)
            mix_hash: Default::default(),
            // KubChain: Nonce is zero (similar to post-merge Ethereum)
            nonce: Default::default(),
            // KubChain: base_fee_per_gas is set when London is active
            base_fee_per_gas: Some(evm_env.block_env.basefee),

            // === Fields KubChain does NOT use ===
            // No Shanghai withdrawals
            withdrawals_root: None,
            // No EIP-4844 blobs
            blob_gas_used: None,
            excess_blob_gas: None,
            // No beacon chain
            parent_beacon_block_root: None,
            // No EIP-7685 requests
            requests_hash: None,
        };

        Ok(Block {
            header,
            body: BlockBody {
                transactions,
                ommers: Default::default(),
                withdrawals: None,
            },
        })
    }
}
