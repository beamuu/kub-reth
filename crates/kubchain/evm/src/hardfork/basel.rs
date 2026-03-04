//! Basel hardfork state migration.
//!
//! At the Basel activation block, deploy upgraded V3 contract bytecodes for
//! the full PoSA contract suite and initialize the SuperNode feature via
//! `StakeManager.initialSuperNode()`.
//!
//! # Placeholder Status
//!
//! Bytecodes are currently empty — fill in the actual compiled bytecodes from
//! the bkc source repository before network activation:
//!
//! ```rust,ignore
//! static STAKE_MANAGER_V3: &[u8] = include_bytes!("bytecodes/basel/stake_manager_v3.bin");
//! static STAKE_MANAGER_STORAGE_V3: &[u8] = include_bytes!("bytecodes/basel/stake_manager_storage_v3.bin");
//! static SLASH_MANAGER_V3: &[u8] = include_bytes!("bytecodes/basel/slash_manager_v3.bin");
//! static NFT_CONTRACT_V3: &[u8] = include_bytes!("bytecodes/basel/nft_contract_v3.bin");
//! static BKC_VALIDATOR_SET_V3: &[u8] = include_bytes!("bytecodes/basel/bkc_validator_set_v3.bin");
//! ```

use alloc::format;
use alloy_evm::block::BlockExecutionError;
use alloy_primitives::Address;
use kubchain_chainspec::KubChainSpec;
use revm::{database::State, database_interface::Database, state::{AccountInfo, Bytecode}};

/// Placeholder: StakeManager V3 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/basel/stake_manager_v3.bin")`
static STAKE_MANAGER_V3: &[u8] = &[];

/// Placeholder: StakeManagerStorage V3 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/basel/stake_manager_storage_v3.bin")`
static STAKE_MANAGER_STORAGE_V3: &[u8] = &[];

/// Placeholder: SlashManager V3 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/basel/slash_manager_v3.bin")`
static SLASH_MANAGER_V3: &[u8] = &[];

/// Placeholder: NFT contract V3 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/basel/nft_contract_v3.bin")`
static NFT_CONTRACT_V3: &[u8] = &[];

/// Placeholder: BKCValidatorSet V3 deployed bytecode.
///
/// TODO: Replace with `include_bytes!("bytecodes/basel/bkc_validator_set_v3.bin")`
static BKC_VALIDATOR_SET_V3: &[u8] = &[];

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
    // TODO: When bytecodes are available, supply the correct on-chain addresses
    // for all contracts. Add stake_manager, stake_manager_storage, slash_manager,
    // and nft_contract fields to KubChainSpec.
    deploy_bytecode(db, chain_spec.validator_contract_v2, BKC_VALIDATOR_SET_V3)?;

    // Commented out until addresses are added to KubChainSpec:
    // deploy_bytecode(db, chain_spec.stake_manager, STAKE_MANAGER_V3)?;
    // deploy_bytecode(db, chain_spec.stake_manager_storage, STAKE_MANAGER_STORAGE_V3)?;
    // deploy_bytecode(db, chain_spec.slash_manager, SLASH_MANAGER_V3)?;
    // deploy_bytecode(db, chain_spec.nft_contract, NFT_CONTRACT_V3)?;

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

    // Suppress unused warnings for placeholder bytecodes
    let _ = STAKE_MANAGER_V3;
    let _ = STAKE_MANAGER_STORAGE_V3;
    let _ = SLASH_MANAGER_V3;
    let _ = NFT_CONTRACT_V3;

    tracing::info!("Basel hardfork applied (placeholder — bytecodes not yet embedded)");
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
