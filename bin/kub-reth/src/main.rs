#![allow(missing_docs)]

#[global_allocator]
static ALLOC: reth_cli_util::allocator::Allocator = reth_cli_util::allocator::new_allocator();

use std::sync::Arc;

use clap::Parser;
use kubchain_chainspec::KubChainSpec;
use kubchain_consensus::KubConsensus;
use kubchain_evm::KubEvmConfig;
use kubchain_node::KubNode;
use reth_cli::chainspec::{parse_genesis, ChainSpecParser};
use reth_ethereum_cli::Cli;
use reth_node_builder::NodeHandle;
use tracing::info;

/// Named KubChain networks.
pub const SUPPORTED_CHAINS: &[&str] = &["kubchain", "kubchain-testnet"];

/// Parse a [`KubChainSpec`] from a named network or a path to a genesis JSON file.
pub fn chain_value_parser(s: &str) -> eyre::Result<Arc<KubChainSpec>> {
    // Future: add KUBCHAIN_MAINNET / KUBCHAIN_TESTNET statics here.
    // For now, all inputs are parsed as genesis JSON paths.
    let _ = s; // suppress unused-var lint while statics are unimplemented
    match s {
        _ => {
            let genesis = parse_genesis(s)?;
            Ok(Arc::new(KubChainSpec::from_genesis(genesis)))
        }
    }
}

/// CLI chain spec parser for KubChain.
#[derive(Debug, Clone, Default)]
pub struct KubChainSpecParser;

impl ChainSpecParser for KubChainSpecParser {
    type ChainSpec = KubChainSpec;
    const SUPPORTED_CHAINS: &'static [&'static str] = SUPPORTED_CHAINS;

    fn parse(s: &str) -> eyre::Result<Arc<KubChainSpec>> {
        chain_value_parser(s)
    }
}

fn main() {
    reth_cli_util::sigsegv_handler::install();

    if std::env::var_os("RUST_BACKTRACE").is_none() {
        unsafe { std::env::set_var("RUST_BACKTRACE", "1") };
    }

    if let Err(err) = Cli::<KubChainSpecParser>::parse().run_with_components::<KubNode>(
        |spec| (KubEvmConfig::new(spec.clone()), Arc::new(KubConsensus::new(spec))),
        async move |builder, _| {
            info!(target: "reth::cli", "Launching KubChain node");
            let NodeHandle { node_exit_future, .. } =
                builder.node(KubNode::default()).launch().await?;
            node_exit_future.await
        },
    ) {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
