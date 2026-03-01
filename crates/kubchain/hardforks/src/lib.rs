//! KubChain hardfork definitions.
//!
//! Defines the custom hardforks specific to KubChain (Bitkub Chain) that extend
//! the standard Ethereum hardfork schedule. These are mixed with [`EthereumHardfork`]
//! entries in the chain's hardfork schedule.
//!
//! # Hardfork History
//!
//! | Fork | Purpose |
//! |------|---------|
//! | Erawan | Modified voting logic for the PoA phase |
//! | Chaophraya | PoSA consensus activation (mainnet) |
//! | ChaophrayaBangkok | PoSA consensus activation (testnet variant) |
//! | Lausanne | StakeManager/SlashManager V2 upgrade |
//! | Basel | SuperNode introduction, V3 contract upgrades |

#[allow(unused_imports)]
use reth_chainspec::{hardfork, ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};

pub mod dev;

hardfork!(
    /// KubChain-specific hardfork identifiers.
    ///
    /// When building a hardfork schedule for KubChain, these are mixed with
    /// [`EthereumHardfork`] entries. Each hardfork is activated at a specific
    /// block number via [`ForkCondition::Block`].
    KubHardfork {
        /// Modified voting logic for the PoA consensus phase.
        ///
        /// Source: `bkc/params/config.go` field `ErawanBlock`
        Erawan,
        /// PoSA (Proof-of-Staked-Authority) consensus activation on mainnet.
        ///
        /// Enables span-based validator rotation, system contract interactions,
        /// reward distribution, and slashing. This is the most significant
        /// consensus change in KubChain's history.
        ///
        /// Source: `bkc/params/config.go` field `ChaophrayaBlock`
        Chaophraya,
        /// PoSA consensus activation on Bangkok testnet.
        ///
        /// Testnet-specific variant with different activation parameters.
        ///
        /// Source: `bkc/params/config.go` field `ChaophrayaBangkokBlock`
        ChaophrayaBangkok,
        /// StakeManager and SlashManager V2 upgrade.
        ///
        /// Deploys V2 contract implementations and updates storage slots
        /// for improved staking parameters (slash threshold, epoch size,
        /// minimum stakes, solo slash rate).
        ///
        /// Source: `bkc/consensus/clique/hardfork/lausanne/instruction.go`
        Lausanne,
        /// SuperNode introduction and V3 contract upgrades.
        ///
        /// Converts official nodes to super nodes, deploys V3 contract versions,
        /// and enables variable block periods. Major architectural change to the
        /// validator set management.
        ///
        /// Source: `bkc/consensus/clique/hardfork/basel/instruction.go`
        Basel,
    }
);

/// Build the KubChain-specific hardfork schedule for a dev/test environment.
///
/// All standard Ethereum forks through London and all custom KubChain forks
/// are activated at block 0.
pub fn kub_dev_hardforks() -> ChainHardforks {
    dev::kub_dev_hardforks()
}
