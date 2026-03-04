//! KubChain chain specification.
//!
//! Defines the chain specification for KubChain (Bitkub Chain), including
//! genesis configuration, hardfork schedule, and chain-specific parameters
//! like span size and validator contract addresses.

use alloy_genesis::Genesis;
use std::{boxed::Box, vec::Vec};
use alloy_consensus::Header;
use alloy_eips::{eip1559::BaseFeeParams, eip7840::BlobParams};
use alloy_primitives::{Address, B256, U256};
use kubchain_primitives::SpanConfig;
use reth_chainspec::{
    ChainSpec, DepositContract, EthChainSpec, EthereumHardforks, ForkFilter, ForkId, Hardforks,
    Head,
};
use reth_ethereum_forks::{EthereumHardfork, ForkCondition, Hardfork};
use std::sync::Arc;

pub mod dev;

/// KubChain chain specification extending reth's [`ChainSpec`] with
/// PoSA-specific configuration.
#[derive(Debug, Clone)]
pub struct KubChainSpec {
    /// Base chain specification (genesis, hardforks, chain ID, etc.).
    pub inner: Arc<ChainSpec>,
    /// Span configuration for validator rotation.
    pub span_config: SpanConfig,
    /// V1 validator set contract address.
    pub validator_contract: Address,
    /// V2 validator set contract address (ChaophrayaBangkok onwards).
    pub validator_contract_v2: Address,
}

impl KubChainSpec {
    /// Returns a reference to the inner [`ChainSpec`].
    pub fn inner(&self) -> &ChainSpec {
        &self.inner
    }

    /// Returns the chain ID.
    pub fn chain_id(&self) -> u64 {
        self.inner.chain.id()
    }

    /// Returns the span size in blocks.
    pub fn span(&self) -> u64 {
        self.span_config.span
    }

    /// Returns the block period in seconds.
    pub fn period(&self) -> u64 {
        self.span_config.period
    }

    /// Construct a [`KubChainSpec`] from a [`Genesis`] JSON descriptor.
    ///
    /// Uses default PoSA span config and zero contract addresses. Intended for
    /// custom genesis files passed via the `--chain` CLI argument.
    pub fn from_genesis(genesis: Genesis) -> Self {
        Self {
            inner: Arc::new(ChainSpec::from(genesis)),
            span_config: SpanConfig { span: 200, period: 3, epoch: 30_000 },
            validator_contract: Address::ZERO,
            validator_contract_v2: Address::ZERO,
        }
    }
}

impl core::ops::Deref for KubChainSpec {
    type Target = ChainSpec;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl EthChainSpec for KubChainSpec {
    type Header = Header;

    fn chain(&self) -> alloy_chains::Chain {
        self.inner.chain()
    }

    fn base_fee_params_at_timestamp(&self, timestamp: u64) -> BaseFeeParams {
        self.inner.base_fee_params_at_timestamp(timestamp)
    }

    fn blob_params_at_timestamp(&self, timestamp: u64) -> Option<BlobParams> {
        self.inner.blob_params_at_timestamp(timestamp)
    }

    fn deposit_contract(&self) -> Option<&DepositContract> {
        self.inner.deposit_contract()
    }

    fn genesis_hash(&self) -> B256 {
        self.inner.genesis_hash()
    }

    fn prune_delete_limit(&self) -> usize {
        self.inner.prune_delete_limit()
    }

    fn display_hardforks(&self) -> Box<dyn core::fmt::Display> {
        self.inner.display_hardforks()
    }

    fn genesis_header(&self) -> &Header {
        self.inner.genesis_header()
    }

    fn genesis(&self) -> &alloy_genesis::Genesis {
        self.inner.genesis()
    }

    fn bootnodes(&self) -> Option<Vec<reth_network_peers::NodeRecord>> {
        self.inner.bootnodes()
    }

    fn final_paris_total_difficulty(&self) -> Option<U256> {
        self.inner.final_paris_total_difficulty()
    }
}

impl Hardforks for KubChainSpec {
    fn fork<H: Hardfork>(&self, fork: H) -> ForkCondition {
        self.inner.fork(fork)
    }

    fn forks_iter(&self) -> impl Iterator<Item = (&dyn Hardfork, ForkCondition)> {
        self.inner.forks_iter()
    }

    fn fork_id(&self, head: &Head) -> ForkId {
        self.inner.fork_id(head)
    }

    fn latest_fork_id(&self) -> ForkId {
        self.inner.latest_fork_id()
    }

    fn fork_filter(&self, head: Head) -> ForkFilter {
        self.inner.fork_filter(head)
    }
}

impl EthereumHardforks for KubChainSpec {
    fn ethereum_fork_activation(&self, fork: EthereumHardfork) -> ForkCondition {
        self.fork(fork)
    }
}
