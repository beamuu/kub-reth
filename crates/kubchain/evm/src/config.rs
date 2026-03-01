//! KubChain SpecId mapping.
//!
//! Maps KubChain hardfork activation to revm [`SpecId`]. KubChain's EVM is
//! capped at London — no Shanghai, Cancun, or Prague features are enabled.

use reth_chainspec::EthereumHardforks;
use reth_primitives_traits::BlockHeader;
use revm::primitives::hardfork::SpecId;

/// Map the latest active hardfork at the given header to a revm [`SpecId`].
///
/// KubChain never goes above [`SpecId::LONDON`]. Post-London features
/// (Shanghai withdrawals, Cancun blobs, etc.) are not part of KubChain.
pub fn kub_revm_spec<C, H>(chain_spec: &C, header: &H) -> SpecId
where
    C: EthereumHardforks,
    H: BlockHeader,
{
    kub_revm_spec_by_block_number(chain_spec, header.number())
}

/// Map the latest active hardfork at the given block number to a revm [`SpecId`].
///
/// KubChain uses block-number-based activations only (no timestamp-based forks).
/// The spec is capped at LONDON — never returns MERGE or later.
pub fn kub_revm_spec_by_block_number<C>(chain_spec: &C, block_number: u64) -> SpecId
where
    C: EthereumHardforks,
{
    // KubChain caps at London. Never activate post-London specs.
    // Check from highest to lowest.
    if chain_spec.is_london_active_at_block(block_number) {
        SpecId::LONDON
    } else if chain_spec.is_berlin_active_at_block(block_number) {
        SpecId::BERLIN
    } else if chain_spec.is_istanbul_active_at_block(block_number) {
        SpecId::ISTANBUL
    } else if chain_spec.is_petersburg_active_at_block(block_number) {
        SpecId::PETERSBURG
    } else if chain_spec.is_byzantium_active_at_block(block_number) {
        SpecId::BYZANTIUM
    } else if chain_spec.is_spurious_dragon_active_at_block(block_number) {
        SpecId::SPURIOUS_DRAGON
    } else if chain_spec.is_tangerine_whistle_active_at_block(block_number) {
        SpecId::TANGERINE
    } else if chain_spec.is_homestead_active_at_block(block_number) {
        SpecId::HOMESTEAD
    } else {
        SpecId::FRONTIER
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::Header;
    use reth_chainspec::ChainSpecBuilder;

    #[test]
    fn test_kub_spec_never_above_london() {
        // Even with Shanghai activated, KubChain should stay at LONDON.
        let chain_spec = ChainSpecBuilder::mainnet().london_activated().build();
        assert_eq!(kub_revm_spec_by_block_number(&chain_spec, 0), SpecId::LONDON);
    }

    #[test]
    fn test_kub_spec_from_header() {
        let chain_spec = ChainSpecBuilder::mainnet().london_activated().build();
        let header = Header::default();
        assert_eq!(kub_revm_spec(&chain_spec, &header), SpecId::LONDON);
    }

    #[test]
    fn test_kub_spec_frontier() {
        let chain_spec = ChainSpecBuilder::mainnet().frontier_activated().build();
        assert_eq!(kub_revm_spec_by_block_number(&chain_spec, 0), SpecId::FRONTIER);
    }

    #[test]
    fn test_kub_spec_istanbul() {
        let chain_spec = ChainSpecBuilder::mainnet().istanbul_activated().build();
        assert_eq!(kub_revm_spec_by_block_number(&chain_spec, 0), SpecId::ISTANBUL);
    }

    #[test]
    fn test_kub_spec_berlin() {
        let chain_spec = ChainSpecBuilder::mainnet().berlin_activated().build();
        assert_eq!(kub_revm_spec_by_block_number(&chain_spec, 0), SpecId::BERLIN);
    }
}
