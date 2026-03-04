//! Typed read-only queries to KubChain system contracts via EVM system calls.
//!
//! Each function performs a staticcall-like query: it executes a system call
//! through the EVM but does **not** commit the resulting state changes, so
//! contract storage is unaffected. This is the correct pattern for reading
//! on-chain data during block finalization.
//!
//! # Usage
//!
//! ```rust,ignore
//! // Query validators (read-only — state not committed)
//! let validators = caller::query_eligible_validators(&mut evm, validator_contract)?;
//!
//! // Query slash status (read-only)
//! let slashed = caller::query_is_signer_slashed(&mut evm, slash_manager, signer, span)?;
//! ```

use alloc::{format, vec::Vec};
use alloy_evm::{block::BlockExecutionError, Evm};
use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use kubchain_contracts::{
    abi::{ISlashManager, IValidatorSet},
    types::{parse_validators, ValidatorSetResult},
};
use kubchain_primitives::{Validator, SYSTEM_ADDRESS};
use revm::context_interface::result::ResultAndState;

/// Query the current validator set and system contract addresses.
///
/// Calls `getValidators()` on the ValidatorSet contract. The resulting
/// state is **not committed** (read-only query).
///
/// Returns the validators, their voting powers, and the addresses of the
/// StakeManager, SlashManager, and OfficialNode contracts.
pub fn query_validators<E>(
    evm: &mut E,
    validator_contract: Address,
) -> Result<ValidatorSetResult, BlockExecutionError>
where
    E: Evm<Error: core::fmt::Display>,
{
    let calldata: Bytes = IValidatorSet::getValidatorsCall {}.abi_encode().into();
    let ResultAndState { result, .. } = evm
        .transact_system_call(SYSTEM_ADDRESS, validator_contract, calldata)
        .map_err(|e| BlockExecutionError::msg(format!("getValidators call failed: {e}")))?;

    let output = result.into_output().unwrap_or_default();
    let ret = IValidatorSet::getValidatorsCall::abi_decode_returns(&output)
        .map_err(|e| BlockExecutionError::msg(format!("getValidators decode failed: {e}")))?;

    Ok(parse_validators(
        ret.validators,
        ret.powers,
        ret.stakeManager,
        ret.slashManager,
        ret.officialNode,
    ))
}

/// Query eligible validators for the next span.
///
/// Calls `getEligibleValidators()` on the ValidatorSet contract. The
/// resulting state is **not committed** (read-only query).
///
/// Returns the list of validators that are eligible to be committed in
/// the next `commitSpan()` call.
pub fn query_eligible_validators<E>(
    evm: &mut E,
    validator_contract: Address,
) -> Result<Vec<Validator>, BlockExecutionError>
where
    E: Evm<Error: core::fmt::Display>,
{
    let calldata: Bytes = IValidatorSet::getEligibleValidatorsCall {}.abi_encode().into();
    let ResultAndState { result, .. } = evm
        .transact_system_call(SYSTEM_ADDRESS, validator_contract, calldata)
        .map_err(|e| {
            BlockExecutionError::msg(format!("getEligibleValidators call failed: {e}"))
        })?;

    let output = result.into_output().unwrap_or_default();
    let ret = IValidatorSet::getEligibleValidatorsCall::abi_decode_returns(&output)
        .map_err(|e| {
            BlockExecutionError::msg(format!("getEligibleValidators decode failed: {e}"))
        })?;

    let validators = ret
        .validators
        .into_iter()
        .zip(ret.powers)
        .map(|(address, voting_power)| Validator { address, voting_power })
        .collect();

    Ok(validators)
}

/// Query the current span number.
///
/// Calls `currentSpanNumber()` on the ValidatorSet contract. The resulting
/// state is **not committed** (read-only query).
pub fn query_current_span<E>(
    evm: &mut E,
    validator_contract: Address,
) -> Result<U256, BlockExecutionError>
where
    E: Evm<Error: core::fmt::Display>,
{
    let calldata: Bytes = IValidatorSet::currentSpanNumberCall {}.abi_encode().into();
    let ResultAndState { result, .. } = evm
        .transact_system_call(SYSTEM_ADDRESS, validator_contract, calldata)
        .map_err(|e| BlockExecutionError::msg(format!("currentSpanNumber call failed: {e}")))?;

    let output = result.into_output().unwrap_or_default();
    let ret = IValidatorSet::currentSpanNumberCall::abi_decode_returns(&output)
        .map_err(|e| BlockExecutionError::msg(format!("currentSpanNumber decode failed: {e}")))?;

    Ok(ret)
}

/// Check if a signer was already slashed in the given span.
///
/// Calls `isSignerSlashed(address, uint256)` on the SlashManager contract.
/// The resulting state is **not committed** (read-only query).
///
/// Returns `true` if the signer has already been slashed for the given span,
/// meaning no additional slash system transaction should be injected.
pub fn query_is_signer_slashed<E>(
    evm: &mut E,
    slash_manager: Address,
    signer: Address,
    span: U256,
) -> Result<bool, BlockExecutionError>
where
    E: Evm<Error: core::fmt::Display>,
{
    let calldata: Bytes =
        ISlashManager::isSignerSlashedCall { signer, span }.abi_encode().into();
    let ResultAndState { result, .. } = evm
        .transact_system_call(SYSTEM_ADDRESS, slash_manager, calldata)
        .map_err(|e| BlockExecutionError::msg(format!("isSignerSlashed call failed: {e}")))?;

    let output = result.into_output().unwrap_or_default();
    let ret = ISlashManager::isSignerSlashedCall::abi_decode_returns(&output)
        .map_err(|e| BlockExecutionError::msg(format!("isSignerSlashed decode failed: {e}")))?;

    Ok(ret)
}
