# KubChain Migration: Architecture & Design

This directory contains the architectural design documents for migrating KubChain (Bitkub Chain) from its legacy geth fork (`bkc`) to `kub-reth` (based on bnb-reth).

## Documents

| Document | Description |
|----------|-------------|
| [migration-overview.md](./migration-overview.md) | High-level migration strategy and compatibility assessment |
| [consensus-design.md](./consensus-design.md) | PoSA consensus module design in Rust |
| [evm-compatibility.md](./evm-compatibility.md) | EVM version matching and hardfork configuration |
| [p2p-compatibility.md](./p2p-compatibility.md) | P2P protocol compatibility analysis |
| [bsc-architecture.md](./bsc-architecture.md) | BSC additions to reth and reuse analysis |
| [crate-structure.md](./crate-structure.md) | Proposed crate layout for kubchain modules |

## Quick Reference

- **Source codebase**: `bkc/` (Go-Ethereum fork, PoSA consensus on Clique)
- **Target codebase**: `kub-reth/` (bnb-reth fork, Rust, modular architecture)
- **EVM level**: London (EIP-1559) - no Shanghai/Cancun
- **P2P protocol**: ETH66 (compatible)
- **Consensus**: Custom PoSA with span-based validator rotation
