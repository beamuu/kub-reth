//! Types returned by system contract calls.

use alloy_primitives::{Address, U256};
use kubchain_primitives::Validator;

/// Result of calling `getValidators()` on the ValidatorSet contract.
#[derive(Debug, Clone)]
pub struct ValidatorSetResult {
    /// Ordered list of validators with their voting power.
    pub validators: Vec<Validator>,
    /// StakeManager contract address.
    pub stake_manager: Address,
    /// SlashManager contract address.
    pub slash_manager: Address,
    /// OfficialNode or SuperNode contract address.
    pub node_contract: Address,
}

/// Converts raw contract return data into a `ValidatorSetResult`.
pub fn parse_validators(
    addresses: Vec<Address>,
    powers: Vec<U256>,
    stake_manager: Address,
    slash_manager: Address,
    node_contract: Address,
) -> ValidatorSetResult {
    let validators = addresses
        .into_iter()
        .zip(powers)
        .map(|(address, voting_power)| Validator {
            address,
            voting_power,
        })
        .collect();

    ValidatorSetResult {
        validators,
        stake_manager,
        slash_manager,
        node_contract,
    }
}
