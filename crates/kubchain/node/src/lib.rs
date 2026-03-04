//! KubChain node type and component assembly.
//!
//! Defines `KubNode` which implements reth's `NodeTypes` trait, providing
//! the complete type configuration for a KubChain execution client.
//!
//! # Architecture
//!
//! `KubNode` assembles the following components via `ComponentsBuilder`:
//!
//! - **Consensus**: `KubConsensus` (PoSA validation)
//! - **EVM**: `KubEvmConfig` (London EVM + system tx injection)
//! - **Pool**: `EthereumPoolBuilder` (standard tx pool, reused)
//! - **Network**: `EthereumNetworkBuilder` (ETH66 P2P, reused)
//! - **Payload**: `EthereumPayloadBuilder` (KubChain uses Ethereum-compatible blocks)
//! - **AddOns**: `EthereumAddOns` (standard ETH JSON-RPC)

use std::sync::Arc;

use eyre::Result;
use kubchain_chainspec::KubChainSpec;
use kubchain_consensus::KubConsensus;
use kubchain_evm::KubEvmConfig;
use reth_ethereum_primitives::EthPrimitives;
use reth_node_builder::{
    components::{
        BasicPayloadServiceBuilder, ComponentsBuilder, ConsensusBuilder, ExecutorBuilder,
    },
    node::{FullNodeTypes, NodeTypes},
    BuilderContext, Node, NodeAdapter,
};
use reth_node_ethereum::{
    node::{
        EthereumAddOns, EthereumEthApiBuilder, EthereumEngineValidatorBuilder,
        EthereumNetworkBuilder, EthereumPayloadBuilder, EthereumPoolBuilder,
    },
    EthEngineTypes,
};
use reth_provider::EthStorage;

/// KubChain node type — marker struct for reth's node builder.
#[derive(Debug, Default, Clone, Copy)]
pub struct KubNode;

impl NodeTypes for KubNode {
    type Primitives = EthPrimitives;
    type ChainSpec = KubChainSpec;
    type Storage = EthStorage;
    type Payload = EthEngineTypes;
}

/// Builds [`KubConsensus`] for the KubChain node.
#[derive(Debug, Default, Clone, Copy)]
pub struct KubConsensusBuilder;

impl<Node> ConsensusBuilder<Node> for KubConsensusBuilder
where
    Node: FullNodeTypes<Types: NodeTypes<ChainSpec = KubChainSpec, Primitives = EthPrimitives>>,
{
    type Consensus = Arc<KubConsensus>;

    async fn build_consensus(self, ctx: &BuilderContext<Node>) -> Result<Self::Consensus> {
        Ok(Arc::new(KubConsensus::new(ctx.chain_spec())))
    }
}

/// Builds [`KubEvmConfig`] for the KubChain node.
#[derive(Debug, Default, Clone, Copy)]
pub struct KubExecutorBuilder;

impl<Types, Node> ExecutorBuilder<Node> for KubExecutorBuilder
where
    Types: NodeTypes<ChainSpec = KubChainSpec, Primitives = EthPrimitives>,
    Node: FullNodeTypes<Types = Types>,
{
    type EVM = KubEvmConfig;

    async fn build_evm(self, ctx: &BuilderContext<Node>) -> Result<Self::EVM> {
        Ok(KubEvmConfig::new(ctx.chain_spec()))
    }
}

impl<N> Node<N> for KubNode
where
    N: FullNodeTypes<Types = Self>,
{
    type ComponentsBuilder = ComponentsBuilder<
        N,
        EthereumPoolBuilder,
        BasicPayloadServiceBuilder<EthereumPayloadBuilder>,
        EthereumNetworkBuilder,
        KubExecutorBuilder,
        KubConsensusBuilder,
    >;

    type AddOns =
        EthereumAddOns<NodeAdapter<N>, EthereumEthApiBuilder, EthereumEngineValidatorBuilder>;

    fn components_builder(&self) -> Self::ComponentsBuilder {
        ComponentsBuilder::default()
            .node_types::<N>()
            .pool(EthereumPoolBuilder::default())
            .executor(KubExecutorBuilder)
            .payload(BasicPayloadServiceBuilder::default())
            .network(EthereumNetworkBuilder::default())
            .consensus(KubConsensusBuilder)
    }

    fn add_ons(&self) -> Self::AddOns {
        EthereumAddOns::default()
    }
}
