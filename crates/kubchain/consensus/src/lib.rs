//! KubChain PoSA Consensus Implementation.
//!
//! Implements reth's consensus traits (`HeaderValidator`, `Consensus`, `FullConsensus`)
//! for KubChain's Proof-of-Staked-Authority (PoSA) consensus engine.
//!
//! # Architecture
//!
//! The consensus engine is based on bkc's modified Clique PoA with PoSA extensions:
//!
//! - **Validator selection**: Weighted random based on staking power
//! - **Span rotation**: Validators rotate every N blocks (configurable span)
//! - **System contracts**: StakeManager, SlashManager, ValidatorSet
//! - **System transactions**: Zero-gas txs for rewards, slashing, span commitment
//!
//! # Validation Phases
//!
//! 1. `HeaderValidator::validate_header` - Verify extra-data, signer, difficulty
//! 2. `HeaderValidator::validate_header_against_parent` - Check timestamp, sequence
//! 3. `Consensus::validate_body_against_header` - Verify tx root, uncle hash
//! 4. `Consensus::validate_block_pre_execution` - Verify signer authorization
//! 5. `FullConsensus::validate_block_post_execution` - Verify state root, receipts

pub mod error;
pub mod extra_data;
pub mod snapshot;
pub mod validation;
pub mod validator;

use alloy_consensus::Header;
use kubchain_chainspec::KubChainSpec;
use kubchain_primitives::{INMEMORY_SIGNATURES, INMEMORY_SNAPSHOTS};
use lru::LruCache;
use parking_lot::RwLock;
use reth_consensus::{Consensus, ConsensusError, FullConsensus, HeaderValidator};
use reth_execution_types::BlockExecutionResult;
use reth_primitives_traits::{NodePrimitives, RecoveredBlock, SealedBlock, SealedHeader};
use snapshot::KubSnapshot;
use std::{num::NonZeroUsize, sync::Arc};

use alloy_primitives::{Address, B256};

/// KubChain PoSA consensus engine.
///
/// Implements header validation, block validation, and post-execution validation
/// for KubChain's Proof-of-Staked-Authority consensus.
///
/// The consensus engine maintains caches for snapshots and signer recovery to
/// avoid redundant computation during block validation.
#[derive(Debug)]
pub struct KubConsensus {
    /// KubChain chain specification.
    chain_spec: Arc<KubChainSpec>,
    /// LRU cache of recent validator snapshots (keyed by block hash).
    snapshot_cache: RwLock<LruCache<B256, KubSnapshot>>,
    /// LRU cache of recently recovered block signers (keyed by block hash).
    signature_cache: RwLock<LruCache<B256, Address>>,
}

impl KubConsensus {
    /// Creates a new `KubConsensus` instance.
    pub fn new(chain_spec: Arc<KubChainSpec>) -> Self {
        Self {
            chain_spec,
            snapshot_cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(INMEMORY_SNAPSHOTS).expect("non-zero"),
            )),
            signature_cache: RwLock::new(LruCache::new(
                NonZeroUsize::new(INMEMORY_SIGNATURES).expect("non-zero"),
            )),
        }
    }

    /// Returns a reference to the chain spec.
    pub fn chain_spec(&self) -> &KubChainSpec {
        &self.chain_spec
    }
}

impl HeaderValidator<Header> for KubConsensus {
    fn validate_header(&self, header: &SealedHeader<Header>) -> Result<(), ConsensusError> {
        validation::validate_header(header, &self.chain_spec)
    }

    fn validate_header_against_parent(
        &self,
        header: &SealedHeader<Header>,
        parent: &SealedHeader<Header>,
    ) -> Result<(), ConsensusError> {
        validation::validate_header_against_parent(header, parent, &self.chain_spec)
    }
}

impl<B> Consensus<B> for KubConsensus
where
    B: reth_primitives_traits::Block<Header = Header>,
{
    type Error = ConsensusError;

    fn validate_body_against_header(
        &self,
        body: &B::Body,
        header: &SealedHeader<B::Header>,
    ) -> Result<(), Self::Error> {
        validation::validate_body_against_header::<B>(body, header)
    }

    fn validate_block_pre_execution(&self, block: &SealedBlock<B>) -> Result<(), Self::Error> {
        validation::validate_block_pre_execution(block, &self.chain_spec)
    }
}

impl<N> FullConsensus<N> for KubConsensus
where
    N: NodePrimitives<Block: reth_primitives_traits::Block<Header = Header>>,
{
    fn validate_block_post_execution(
        &self,
        block: &RecoveredBlock<N::Block>,
        result: &BlockExecutionResult<N::Receipt>,
    ) -> Result<(), ConsensusError> {
        validation::validate_block_post_execution::<N>(block, result, &self.chain_spec)
    }
}
