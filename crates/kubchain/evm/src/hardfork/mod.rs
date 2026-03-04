//! KubChain hardfork state migrations.
//!
//! At each hardfork activation block, the consensus layer must apply
//! on-chain state migrations before executing user transactions. These
//! migrations deploy upgraded system contract bytecodes and update
//! storage layout changes.
//!
//! # Hardforks
//!
//! - **Lausanne**: Deploys V2 bytecodes for StakeManager, StakeManagerStorage,
//!   and SlashManager. Updates storage slots for new PoSA parameters.
//!
//! - **Basel**: Deploys V3 bytecodes for the full contract suite and
//!   initializes the SuperNode feature.
//!
//! # Placeholder Status
//!
//! The actual compiled contract bytecodes from the bkc source repository have
//! not yet been embedded. The migration functions log a warning and skip
//! bytecode deployment when `bytecodes` are empty. Fill in the `static` arrays
//! in each module once the bytecodes are available.

pub mod basel;
pub mod lausanne;

pub use basel::apply_basel_hardfork;
pub use lausanne::apply_lausanne_hardfork;
