//! Development and testing hardfork schedules for KubChain.

use crate::KubHardfork;
use reth_chainspec::{ChainHardforks, EthereumHardfork, ForkCondition, Hardfork};

/// Returns a hardfork schedule with all forks activated at block 0.
///
/// Useful for development and testing where all features should be
/// immediately available.
pub fn kub_dev_hardforks() -> ChainHardforks {
    ChainHardforks::new(vec![
        // Standard Ethereum forks (all at block 0)
        (EthereumHardfork::Frontier.boxed(), ForkCondition::Block(0)),
        (
            EthereumHardfork::Homestead.boxed(),
            ForkCondition::Block(0),
        ),
        (EthereumHardfork::Dao.boxed(), ForkCondition::Block(0)),
        (
            EthereumHardfork::Tangerine.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::SpuriousDragon.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::Byzantium.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::Constantinople.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::Petersburg.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::Istanbul.boxed(),
            ForkCondition::Block(0),
        ),
        (
            EthereumHardfork::MuirGlacier.boxed(),
            ForkCondition::Block(0),
        ),
        (EthereumHardfork::Berlin.boxed(), ForkCondition::Block(0)),
        (EthereumHardfork::London.boxed(), ForkCondition::Block(0)),
        (
            EthereumHardfork::ArrowGlacier.boxed(),
            ForkCondition::Block(0),
        ),
        // NOTE: No Shanghai, Cancun, or Prague - KubChain stops at London/ArrowGlacier
        // KubChain custom forks (all at block 0 for dev)
        (KubHardfork::Erawan.boxed(), ForkCondition::Block(0)),
        (KubHardfork::Chaophraya.boxed(), ForkCondition::Block(0)),
        (
            KubHardfork::ChaophrayaBangkok.boxed(),
            ForkCondition::Block(0),
        ),
        (KubHardfork::Lausanne.boxed(), ForkCondition::Block(0)),
        (KubHardfork::Basel.boxed(), ForkCondition::Block(0)),
    ])
}
