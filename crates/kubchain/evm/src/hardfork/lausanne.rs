//! Lausanne hardfork state migration.
//!
//! At the Lausanne activation block, deploy upgraded V2 contract bytecodes
//! for the core PoSA system contracts: StakeManager, StakeManagerStorage,
//! and SlashManager.
//!
//! # How to fill in bytecodes
//!
//! Paste the hex-encoded deployed bytecode (without `0x` prefix) from the
//! compiled contracts into the constants below. These are the *deployed*
//! bytecodes (what `eth_getCode` returns), not the constructor bytecodes.
//!
//! ```rust,ignore
//! // Example — replace the empty string with actual hex:
//! const STAKE_MANAGER_V2_HEX: &str = "608060405234801561001057600080fd5b50...";
//! ```

use alloc::format;
use alloy_evm::block::BlockExecutionError;
use alloy_primitives::{hex, Address};
use kubchain_chainspec::KubChainSpec;
use revm::{database::State, database_interface::Database, state::{AccountInfo, Bytecode}};

// ---------------------------------------------------------------------------
// Bytecodes — paste hex-encoded deployed bytecode (no 0x prefix) here.
// ---------------------------------------------------------------------------

/// Deployed bytecode of StakeManager V2 (Lausanne upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
/// Example: `"608060405234801561001057600080fd5b50..."` (no `0x` prefix)
const STAKE_MANAGER_V2_HEX: &str = "";

/// Deployed bytecode of StakeManagerStorage V2 (Lausanne upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const STAKE_MANAGER_STORAGE_V2_HEX: &str = "";

/// Deployed bytecode of SlashManager V2 (Lausanne upgrade).
///
/// Paste the hex string from `solc --bin-runtime` or `eth_getCode` output.
const SLASH_MANAGER_V2_HEX: &str = "";

/// Apply the Lausanne hardfork state migration.
///
/// Deploys V2 bytecodes for the core PoSA system contracts. Must be called
/// exactly once, at the Lausanne activation block, **before** executing user
/// transactions.
///
/// When bytecodes are empty placeholders, deployment is skipped with a warning
/// log and the function returns `Ok(())`.
pub fn apply_lausanne_hardfork<DB>(
    db: &mut State<DB>,
    chain_spec: &KubChainSpec,
) -> Result<(), BlockExecutionError>
where
    DB: Database,
{
    tracing::info!(
        validator_contract = %chain_spec.validator_contract,
        "Applying Lausanne hardfork state migration (V2 contracts)"
    );

    // Deploy V2 bytecodes for all three core PoSA contracts.
    //
    // TODO: When contract addresses are added to KubChainSpec, uncomment
    // and pass the correct addresses (stake_manager, stake_manager_storage,
    // slash_manager) instead of validator_contract placeholder.
    deploy_bytecode(db, chain_spec.validator_contract, STAKE_MANAGER_V2_HEX)?;

    // Commented out until addresses are added to KubChainSpec:
    // deploy_bytecode(db, chain_spec.stake_manager, STAKE_MANAGER_V2_HEX)?;
    // deploy_bytecode(db, chain_spec.stake_manager_storage, STAKE_MANAGER_STORAGE_V2_HEX)?;
    // deploy_bytecode(db, chain_spec.slash_manager, SLASH_MANAGER_V2_HEX)?;

    // TODO: Apply StakeManagerStorage V2 storage slot changes.
    // Slot layout changes from bkc source (lausanne migration):
    //   slot 24: soloSlashRate
    //   slot 25: minimumPoolStake
    //   slot 26: autoClaimRewardThreshold
    //   slot 27: feeForValidator
    // Use `db.storage(addr, slot)` to read existing values and
    // update state directly once the address is known.

    tracing::info!("Lausanne hardfork applied (placeholder — bytecodes not yet embedded)");
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
