//! KubChain chain specification.
//!
//! Defines the chain specification for KubChain (Bitkub Chain), including
//! genesis configuration, hardfork schedule, and chain-specific parameters
//! like span size and validator contract addresses.

use alloy_primitives::Address;
use kubchain_primitives::SpanConfig;
use reth_chainspec::ChainSpec;
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
}

impl core::ops::Deref for KubChainSpec {
    type Target = ChainSpec;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
