//! Development chain specification for KubChain.
//!
//! Provides a dev-mode chain spec with all hardforks activated at block 0
//! for local development and testing.

use crate::KubChainSpec;
use alloy_primitives::Address;
use kubchain_hardforks::kub_dev_hardforks;
use kubchain_primitives::SpanConfig;
use reth_chainspec::ChainSpecBuilder;
use std::sync::Arc;

/// Returns a development chain specification with all forks at block 0.
///
/// This is intended for local testing and development. The chain ID is 25925
/// (KubChain testnet). System contract addresses are set to zero and should
/// be overridden with actual deployed contract addresses when used with a
/// real genesis file.
pub fn kub_dev_chain_spec() -> KubChainSpec {
    let chain_spec = ChainSpecBuilder::default()
        .chain(25925u64.into())
        .genesis(Default::default())
        .with_forks(kub_dev_hardforks())
        .build();

    KubChainSpec {
        inner: Arc::new(chain_spec),
        span_config: SpanConfig {
            span: 200,
            period: 5,
            epoch: 30_000,
        },
        validator_contract: Address::ZERO,
        validator_contract_v2: Address::ZERO,
    }
}
