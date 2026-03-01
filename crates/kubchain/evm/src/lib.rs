//! KubChain EVM configuration.
//!
//! Implements reth's [`ConfigureEvm`] trait for KubChain, handling:
//!
//! - EVM spec selection (capped at London/ArrowGlacier)
//! - Fee routing to SystemAddress (when Chaophraya is active)
//! - System transaction injection during block finalization
//! - Hardfork state migrations (Lausanne, Basel)
//!
//! # Block Execution Flow
//!
//! 1. `apply_pre_execution_changes()` — hardfork migrations if activation block
//! 2. Execute user transactions — fees accumulate at SystemAddress
//! 3. `finish()` — inject system transactions:
//!    a. `commitSpan()` at span commitment blocks
//!    b. `slash()` for out-of-turn blocks by official/super nodes
//!    c. `distributeReward()` every block (moves fees from SystemAddress → StakeManager)
//!
//! # Architecture
//!
//! ```text
//! KubEvmConfig (ConfigureEvm)
//!   ├── KubBlockExecutorFactory (BlockExecutorFactory)
//!   │     └── KubBlockExecutor (BlockExecutor)
//!   │           ├── apply_pre_execution_changes: hardfork migrations
//!   │           ├── execute_transaction: standard EVM with fees → SystemAddress
//!   │           └── finish: system tx injection (commitSpan, slash, distributeReward)
//!   └── KubBlockAssembler (BlockAssembler)
//!         └── assemble_block: KubChain-specific header fields
//! ```

#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::sync::Arc;
use alloy_consensus::Header;
use alloy_evm::{EthEvmFactory, FromRecoveredTx, FromTxWithEncoded};
use alloy_primitives::{Address, U256};
use core::{convert::Infallible, fmt::Debug};
use kubchain_chainspec::KubChainSpec;
use kubchain_hardforks::KubHardfork;
use kubchain_primitives::{SystemContracts, SYSTEM_ADDRESS};
use reth_chainspec::{EthChainSpec as _, Hardforks};
use reth_ethereum_primitives::{EthPrimitives, TransactionSigned};

// Ensure these crate dependencies are recognized even if only used in submodules.
use alloy_eips as _;
use alloy_rlp as _;
use reth_execution_types as _;
use reth_evm::{
    precompiles::PrecompilesMap, ConfigureEvm, EvmEnv, EvmFactory, TransactionEnv,
};
use reth_primitives_traits::{SealedBlock, SealedHeader};
use revm::{
    context::{BlockEnv, CfgEnv},
    primitives::hardfork::SpecId,
};

pub mod build;
pub mod config;
pub mod execute;
pub mod system_tx;

pub use build::KubBlockAssembler;
pub use config::{kub_revm_spec, kub_revm_spec_by_block_number};
pub use execute::{KubBlockExecutionCtx, KubBlockExecutor, KubBlockExecutorFactory};

use alloy_consensus::Block;

/// Attributes for configuring the next KubChain block.
///
/// KubChain blocks are simpler than Ethereum — no beacon root, no withdrawals,
/// no prev_randao. The consensus layer provides:
/// - Timestamp (from block period configuration)
/// - Fee recipient (block producer / coinbase)
/// - Gas limit
#[derive(Debug, Clone)]
pub struct KubNextBlockEnvCtx {
    /// Block timestamp.
    pub timestamp: u64,
    /// Block producer address (coinbase).
    pub suggested_fee_recipient: Address,
    /// Block gas limit.
    pub gas_limit: u64,
}

/// KubChain EVM configuration.
///
/// Bundles the block executor factory and block assembler, implementing
/// [`ConfigureEvm`] to integrate with reth's block execution pipeline.
///
/// # Fee Routing
///
/// When the Chaophraya hardfork is active, `block_env.beneficiary` is set to
/// [`SYSTEM_ADDRESS`] so that all gas fees accumulate there. During
/// finalization, the balance is moved to the validator via `distributeReward()`.
#[derive(Debug, Clone)]
pub struct KubEvmConfig<EvmF = EthEvmFactory> {
    /// Block executor factory.
    pub executor_factory: KubBlockExecutorFactory<EvmF>,
    /// Block assembler.
    pub block_assembler: KubBlockAssembler,
    /// Chain specification.
    chain_spec: Arc<KubChainSpec>,
}

impl KubEvmConfig {
    /// Creates a new [`KubEvmConfig`] with the given chain specification.
    pub fn new(chain_spec: Arc<KubChainSpec>) -> Self {
        Self::with_evm_factory(chain_spec, EthEvmFactory::default())
    }
}

impl<EvmF> KubEvmConfig<EvmF> {
    /// Creates a new [`KubEvmConfig`] with a custom EVM factory.
    pub fn with_evm_factory(chain_spec: Arc<KubChainSpec>, evm_factory: EvmF) -> Self {
        Self {
            block_assembler: KubBlockAssembler::new(chain_spec.clone()),
            executor_factory: KubBlockExecutorFactory::new(chain_spec.clone(), evm_factory),
            chain_spec,
        }
    }

    /// Returns the chain specification.
    pub fn chain_spec(&self) -> &Arc<KubChainSpec> {
        &self.chain_spec
    }
}

impl<EvmF> ConfigureEvm for KubEvmConfig<EvmF>
where
    EvmF: EvmFactory<
            Tx: TransactionEnv
                    + FromRecoveredTx<TransactionSigned>
                    + FromTxWithEncoded<TransactionSigned>,
            Spec = SpecId,
            Precompiles = PrecompilesMap,
        > + Clone
        + Debug
        + Send
        + Sync
        + Unpin
        + 'static,
{
    type Primitives = EthPrimitives;
    type Error = Infallible;
    type NextBlockEnvCtx = KubNextBlockEnvCtx;
    type BlockExecutorFactory = KubBlockExecutorFactory<EvmF>;
    type BlockAssembler = KubBlockAssembler;

    fn block_executor_factory(&self) -> &Self::BlockExecutorFactory {
        &self.executor_factory
    }

    fn block_assembler(&self) -> &Self::BlockAssembler {
        &self.block_assembler
    }

    fn evm_env(&self, header: &Header) -> EvmEnv {
        let spec = config::kub_revm_spec(self.chain_spec.inner(), header);

        let cfg_env = CfgEnv::new()
            .with_chain_id(self.chain_spec.chain_id())
            .with_spec(spec);

        // KubChain: After Chaophraya, fees go to SystemAddress.
        // Set beneficiary = SystemAddress so the EVM routes fees there.
        let beneficiary = if self
            .chain_spec
            .inner()
            .fork(KubHardfork::Chaophraya)
            .active_at_block(header.number)
        {
            SYSTEM_ADDRESS
        } else {
            header.beneficiary
        };

        let block_env = BlockEnv {
            number: U256::from(header.number),
            beneficiary,
            timestamp: U256::from(header.timestamp),
            // KubChain: difficulty is non-zero (in-turn=2, out-of-turn=1)
            difficulty: header.difficulty,
            // KubChain: No prevrandao (not PoS merge)
            prevrandao: None,
            gas_limit: header.gas_limit,
            basefee: header.base_fee_per_gas.unwrap_or_default(),
            // KubChain: No EIP-4844 blob gas
            blob_excess_gas_and_price: None,
        };

        EvmEnv { cfg_env, block_env }
    }

    fn next_evm_env(
        &self,
        parent: &Header,
        attributes: &KubNextBlockEnvCtx,
    ) -> Result<EvmEnv, Self::Error> {
        let next_block_number = parent.number + 1;
        let spec = config::kub_revm_spec_by_block_number(
            self.chain_spec.inner(),
            next_block_number,
        );

        let cfg_env = CfgEnv::new()
            .with_chain_id(self.chain_spec.chain_id())
            .with_spec(spec);

        // KubChain: After Chaophraya, fees go to SystemAddress
        let beneficiary = if self
            .chain_spec
            .inner()
            .fork(KubHardfork::Chaophraya)
            .active_at_block(next_block_number)
        {
            SYSTEM_ADDRESS
        } else {
            attributes.suggested_fee_recipient
        };

        // Calculate next block base fee from parent
        let basefee = self
            .chain_spec
            .inner()
            .next_block_base_fee(parent, attributes.timestamp)
            .unwrap_or_default();

        let block_env = BlockEnv {
            number: U256::from(next_block_number),
            beneficiary,
            timestamp: U256::from(attributes.timestamp),
            difficulty: U256::ZERO, // Will be set by consensus layer
            prevrandao: None,
            gas_limit: attributes.gas_limit,
            basefee,
            blob_excess_gas_and_price: None,
        };

        Ok(EvmEnv { cfg_env, block_env })
    }

    fn context_for_block<'a>(
        &self,
        block: &'a SealedBlock<Block<TransactionSigned>>,
    ) -> KubBlockExecutionCtx {
        KubBlockExecutionCtx {
            parent_hash: block.header().parent_hash,
            chain_spec: self.chain_spec.clone(),
            // Signer recovery would happen in the consensus layer.
            // During sync, the signer is pre-verified by consensus.
            // We use the beneficiary as a placeholder here.
            signer: block.header().beneficiary,
            system_contracts: SystemContracts::default(),
        }
    }

    fn context_for_next_block(
        &self,
        parent: &SealedHeader,
        attributes: Self::NextBlockEnvCtx,
    ) -> KubBlockExecutionCtx {
        KubBlockExecutionCtx {
            parent_hash: parent.hash(),
            chain_spec: self.chain_spec.clone(),
            signer: attributes.suggested_fee_recipient,
            system_contracts: SystemContracts::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kubchain_chainspec::dev::kub_dev_chain_spec;

    #[test]
    fn test_kub_evm_config_creation() {
        let chain_spec = Arc::new(kub_dev_chain_spec());
        let config = KubEvmConfig::new(chain_spec.clone());
        assert_eq!(config.chain_spec().chain_id(), 25925);
    }

    #[test]
    fn test_evm_env_with_system_address() {
        let chain_spec = Arc::new(kub_dev_chain_spec());
        let config = KubEvmConfig::new(chain_spec);

        // Dev chain has all forks at block 0, so Chaophraya is active.
        let header = Header {
            number: 100,
            ..Default::default()
        };
        let env = config.evm_env(&header);

        // Beneficiary should be SystemAddress when Chaophraya is active.
        assert_eq!(env.block_env.beneficiary, SYSTEM_ADDRESS);
    }

    #[test]
    fn test_evm_env_spec_capped_at_london() {
        let chain_spec = Arc::new(kub_dev_chain_spec());
        let config = KubEvmConfig::new(chain_spec);

        let header = Header {
            number: 100,
            ..Default::default()
        };
        let env = config.evm_env(&header);

        // Spec should be capped at LONDON
        assert_eq!(env.cfg_env.spec, SpecId::LONDON);
    }

    #[test]
    fn test_next_evm_env() {
        let chain_spec = Arc::new(kub_dev_chain_spec());
        let config = KubEvmConfig::new(chain_spec);

        let parent = Header {
            number: 99,
            gas_limit: 30_000_000,
            ..Default::default()
        };

        let attrs = KubNextBlockEnvCtx {
            timestamp: 100,
            suggested_fee_recipient: Address::ZERO,
            gas_limit: 30_000_000,
        };

        let env = config.next_evm_env(&parent, &attrs).unwrap();

        // Block number should be parent + 1
        assert_eq!(env.block_env.number, U256::from(100));
        // Beneficiary should be SystemAddress (Chaophraya active at block 0)
        assert_eq!(env.block_env.beneficiary, SYSTEM_ADDRESS);
        // Spec should be LONDON
        assert_eq!(env.cfg_env.spec, SpecId::LONDON);
    }
}
