//! Basel hardfork state migration.
//!
//! At the Basel activation block, deploy upgraded V3 contract bytecodes for
//! the full PoSA contract suite and initialize the SuperNode feature via
//! `StakeManager.initialSuperNode()`.
//!
//! # How to fill in bytecodes
//!
//! Paste the hex-encoded deployed bytecode (without `0x` prefix) from the
//! compiled contracts into the constants below. These are the *deployed*
//! bytecodes (what `eth_getCode` returns), not the constructor bytecodes.
//!
//! ```rust,ignore
//! // Example — replace the empty string with actual hex:
//! const STAKE_MANAGER_V3_HEX: &str = "608060405234801561001057600080fd5b50...";
//! ```

use alloc::format;
use alloy_evm::block::BlockExecutionError;
use alloy_primitives::{hex, Address};
use kubchain_chainspec::KubChainSpec;
use revm::{database::State, database_interface::Database, state::{AccountInfo, Bytecode}};

// ---------------------------------------------------------------------------
// Bytecodes — paste hex-encoded deployed bytecode (no 0x prefix) here.
// ---------------------------------------------------------------------------

/// Deployed bytecode of StakeManager V3 (Basel upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const STAKE_MANAGER_V3_HEX: &str = "";

/// Deployed bytecode of StakeManagerStorage V3 (Basel upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const STAKE_MANAGER_STORAGE_V3_HEX: &str = "";

/// Deployed bytecode of SlashManager V3 (Basel upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const SLASH_MANAGER_V3_HEX: &str = "";

/// Deployed bytecode of the NFT contract V3 (Basel upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const NFT_CONTRACT_V3_HEX: &str = "";

/// Deployed bytecode of BKCValidatorSet V3 (Basel upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const BKC_VALIDATOR_SET_V3_HEX: &str = "";

/// Apply the Basel hardfork state migration.
///
/// Deploys V3 bytecodes for the full PoSA contract suite. Must be called
/// exactly once, at the Basel activation block, **before** executing user
/// transactions.
///
/// After bytecode deployment, calls `StakeManager.initialSuperNode()` to
/// initialize the SuperNode feature introduced in Basel.
///
/// When bytecodes are empty placeholders, deployment is skipped with a warning
/// log and the function returns `Ok(())`.
pub fn apply_basel_hardfork<DB>(
    db: &mut State<DB>,
    chain_spec: &KubChainSpec,
) -> Result<(), BlockExecutionError>
where
    DB: Database,
{
    tracing::info!(
        validator_contract_v2 = %chain_spec.validator_contract_v2,
        "Applying Basel hardfork state migration (V3 contracts + SuperNode init)"
    );

    // Deploy V3 bytecodes for all PoSA contracts.
    //
    // TODO: When contract addresses are added to KubChainSpec, uncomment
    // the lines below and pass the correct addresses.
    deploy_bytecode(db, chain_spec.validator_contract_v2, BKC_VALIDATOR_SET_V3_HEX)?;

    // Commented out until addresses are added to KubChainSpec:
    // deploy_bytecode(db, chain_spec.stake_manager, STAKE_MANAGER_V3_HEX)?;
    // deploy_bytecode(db, chain_spec.stake_manager_storage, STAKE_MANAGER_STORAGE_V3_HEX)?;
    // deploy_bytecode(db, chain_spec.slash_manager, SLASH_MANAGER_V3_HEX)?;
    // deploy_bytecode(db, chain_spec.nft_contract, NFT_CONTRACT_V3_HEX)?;

    // TODO: Call initialSuperNode(superNodeAddress) on the StakeManager via EVM.
    // This requires an EVM reference (not just the DB). When the EVM is passed
    // in, use:
    //
    //   use kubchain_contracts::abi::IStakeManager;
    //   use alloy_sol_types::SolCall;
    //   let calldata = IStakeManager::initialSuperNodeCall {
    //       superNodeAddress: chain_spec.super_node,
    //   }.abi_encode().into();
    //   let ResultAndState { result: _, state } = evm
    //       .transact_system_call(coinbase, stake_manager, calldata)
    //       .map_err(|e| BlockExecutionError::msg(format!("initialSuperNode: {e}")))?;
    //   evm.db_mut().commit(state);

    // Suppress unused warnings for placeholder constants
    let _ = STAKE_MANAGER_V3_HEX;
    let _ = STAKE_MANAGER_STORAGE_V3_HEX;
    let _ = SLASH_MANAGER_V3_HEX;
    let _ = NFT_CONTRACT_V3_HEX;

    tracing::info!("Basel hardfork applied (placeholder — bytecodes not yet embedded)");
    Ok(())
}

/// Deploy a contract bytecode (hex string) at the given address, preserving
/// existing balance and nonce.
///
/// `bytecode_hex` is the deployed bytecode as a lowercase hex string without
/// a `0x` prefix (i.e. the output of `solc --bin-runtime` or `eth_getCode`).
///
/// If `bytecode_hex` is empty (placeholder not yet filled in), logs a warning
/// and returns `Ok(())` without modifying state.
fn deploy_bytecode<DB: Database>(
    db: &mut State<DB>,
    address: Address,
    bytecode_hex: &str,
) -> Result<(), BlockExecutionError> {
    if bytecode_hex.is_empty() {
        tracing::warn!(
            %address,
            "Hardfork bytecode deployment skipped — hex constant not yet filled in"
        );
        return Ok(());
    }

    let bytes = hex::decode(bytecode_hex)
        .map_err(|e| BlockExecutionError::msg(format!("Invalid bytecode hex for {address}: {e}")))?;

    let bytecode = Bytecode::new_raw(bytes.into());
    let code_hash = bytecode.hash_slow();

    // Preserve existing balance and nonce.
    let existing = db
        .basic(address)
        .map_err(|_| BlockExecutionError::msg(format!("Failed to load account {address}")))?
        .unwrap_or_default();

    db.insert_account(
        address,
        AccountInfo { balance: existing.balance, nonce: existing.nonce, code_hash, code: Some(bytecode) },
    );

    tracing::info!(%address, "Deployed hardfork contract bytecode");
    Ok(())
}
