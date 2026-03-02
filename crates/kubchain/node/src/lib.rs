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
//! - **Payload**: Custom payload builder with system tx support

// TODO: Implement KubNode and component builders in Phase 5
// This will wire together all kubchain crates into a functional node.
//
// Key reference files:
// - kub-reth/crates/ethereum/node/src/node.rs (EthereumNode pattern)
// - kub-reth/bin/reth/src/main.rs (entry point pattern)
