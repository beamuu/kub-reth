//! Lausanne hardfork state migration.
//!
//! At the Lausanne activation block, deploy upgraded V2 contract bytecodes
//! for the core PoSA system contracts: StakeManager, StakeManagerStorage,
//! and SlashManager.
//!
//! # Placeholder Status
//!
//! Bytecodes are currently empty — fill in the actual compiled bytecodes from
//! the bkc source repository before network activation:
//!
//! ```rust,ignore
//! // Replace the empty slices with:
//! static STAKE_MANAGER_V2: &[u8] = include_bytes!("bytecodes/lausanne/stake_manager_v2.bin");
//! static STAKE_MANAGER_STORAGE_V2: &[u8] = include_bytes!("bytecodes/lausanne/stake_manager_storage_v2.bin");
//! static SLASH_MANAGER_V2: &[u8] = include_bytes!("bytecodes/lausanne/slash_manager_v2.bin");
//! ```

use alloc::format;
use alloy_evm::block::BlockExecutionError;
use alloy_primitives::Address;
use kubchain_chainspec::KubChainSpec;
use revm::{database::State, database_interface::Database, state::{AccountInfo, Bytecode}};

/// Placeholder: StakeManager V2 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/lausanne/stake_manager_v2.bin")`
/// once the bkc source bytecodes are available.
static STAKE_MANAGER_V2: &[u8] = &[];

/// Placeholder: StakeManagerStorage V2 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/lausanne/stake_manager_storage_v2.bin")`
#[allow(dead_code)]
static STAKE_MANAGER_STORAGE_V2: &[u8] = &[];

/// Placeholder: SlashManager V2 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/lausanne/slash_manager_v2.bin")`
#[allow(dead_code)]
static SLASH_MANAGER_V2: &[u8] = &[];

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
    // TODO: When bytecodes are available, supply the correct on-chain addresses
    // for StakeManagerStorage and SlashManager (currently not in KubChainSpec).
    // For now we use validator_contract as a placeholder address.
    deploy_bytecode(db, chain_spec.validator_contract, STAKE_MANAGER_V2)?;

    // Commented out until addresses are added to KubChainSpec:
    // deploy_bytecode(db, chain_spec.stake_manager, STAKE_MANAGER_V2)?;
    // deploy_bytecode(db, chain_spec.stake_manager_storage, STAKE_MANAGER_STORAGE_V2)?;
    // deploy_bytecode(db, chain_spec.slash_manager, SLASH_MANAGER_V2)?;

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

/// Deploy a contract bytecode at the given address, preserving existing
/// balance and nonce.
///
/// If `bytecode_bytes` is empty (placeholder), logs a warning and returns
/// `Ok(())` without modifying state.
fn deploy_bytecode<DB: Database>(
    db: &mut State<DB>,
    address: Address,
    bytecode_bytes: &[u8],
) -> Result<(), BlockExecutionError> {
    if bytecode_bytes.is_empty() {
        tracing::warn!(
            %address,
            "Hardfork bytecode deployment skipped — placeholder empty"
        );
        return Ok(());
    }

    let bytecode = Bytecode::new_raw(bytecode_bytes.to_vec().into());
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
