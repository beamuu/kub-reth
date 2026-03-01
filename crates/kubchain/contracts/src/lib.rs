//! KubChain system contract ABI bindings and interaction layer.
//!
//! Provides Rust bindings for the KubChain system contracts used by the PoSA
//! consensus engine:
//!
//! - **StakeManager**: Staking, reward distribution, vault management
//! - **ValidatorSet**: Validator list, span management, commitment
//! - **SlashManager**: Slashing penalties, threshold tracking

pub mod abi;
pub mod types;
