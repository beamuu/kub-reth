//! KubChain block executor factory and block executor.
//!
//! Implements [`BlockExecutorFactory`] and [`BlockExecutor`] for KubChain's PoSA
//! consensus, handling:
//!
//! - Hardfork state migrations in `apply_pre_execution_changes()`
//! - Standard transaction execution with fees routed to SystemAddress
//! - System transaction injection in `finish()` (commitSpan, slash, distributeReward)

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use alloy_consensus::Transaction;
use alloy_eips::Encodable2718;
use alloy_evm::{
    block::{
        BlockExecutionError, BlockExecutionResult, BlockExecutor, BlockExecutorFactory,
        BlockExecutorFor, BlockValidationError, CommitChanges, ExecutableTx, OnStateHook,
    },
    Database, Evm, EvmFactory, FromRecoveredTx, FromTxWithEncoded,
};
use alloy_primitives::{Address, B256};
use kubchain_chainspec::KubChainSpec;
use kubchain_hardforks::KubHardfork;
use kubchain_primitives::{is_span_commitment_block, SystemContracts, DIFF_NO_TURN};
use reth_chainspec::{EthereumHardforks, Hardforks};
use reth_ethereum_primitives::{Receipt, TransactionSigned};
use reth_evm::{precompiles::PrecompilesMap, TransactionEnv};
use revm::{
    context::result::ExecutionResult, context_interface::result::ResultAndState, database::State,
    DatabaseCommit, Inspector,
};

extern crate alloc;

/// Context for KubChain block execution.
///
/// Contains chain-specific data needed during block execution that goes
/// beyond what the EVM environment provides.
///
/// Uses `Arc<KubChainSpec>` rather than a reference to avoid lifetime
/// coupling with the `ConfigureEvm` instance (the trait's
/// `context_for_block` does not guarantee `&'a self`).
#[derive(Debug, Clone)]
pub struct KubBlockExecutionCtx {
    /// Parent block hash.
    pub parent_hash: B256,
    /// Chain specification (Arc-wrapped for cheap cloning).
    pub chain_spec: Arc<KubChainSpec>,
    /// Current block signer recovered from header extra-data.
    pub signer: Address,
    /// System contract addresses from the current snapshot.
    pub system_contracts: SystemContracts,
}

/// KubChain block executor.
///
/// Executes transactions within a block, applying KubChain-specific pre- and
/// post-execution changes:
///
/// **Pre-execution:**
/// - Set state clear flag (Spurious Dragon)
/// - Apply hardfork state migrations (Lausanne, Basel) if activation block
///
/// **Post-execution (finish):**
/// - Inject `commitSpan()` system tx at span commitment blocks
/// - Inject `slash()` system tx if out-of-turn by official/super node
/// - Inject `distributeReward()` system tx (every block, transfers fees)
#[expect(missing_debug_implementations)]
pub struct KubBlockExecutor<E> {
    /// Execution context (contains chain_spec via Arc).
    ctx: KubBlockExecutionCtx,
    /// Inner EVM instance.
    evm: E,
    /// Accumulated receipts.
    receipts: Vec<Receipt>,
    /// Total gas used.
    gas_used: u64,
    /// Optional state change hook.
    _state_hook: Option<Box<dyn OnStateHook>>,
}

impl<E> KubBlockExecutor<E> {
    /// Creates a new [`KubBlockExecutor`].
    pub fn new(evm: E, ctx: KubBlockExecutionCtx) -> Self {
        Self {
            ctx,
            evm,
            receipts: Vec::new(),
            gas_used: 0,
            _state_hook: None,
        }
    }

    /// Returns the chain spec from the execution context.
    fn chain_spec(&self) -> &KubChainSpec {
        &self.ctx.chain_spec
    }
}

impl<'db, DB, E> BlockExecutor for KubBlockExecutor<E>
where
    DB: Database + 'db,
    E: Evm<
        DB = &'db mut State<DB>,
        Tx: FromRecoveredTx<TransactionSigned> + FromTxWithEncoded<TransactionSigned>,
    >,
{
    type Transaction = TransactionSigned;
    type Receipt = Receipt;
    type Evm = E;

    fn apply_pre_execution_changes(&mut self) -> Result<(), BlockExecutionError> {
        let block_number: u64 = self.evm.block().number.saturating_to();

        // Set state clear flag if Spurious Dragon is active.
        let state_clear_flag =
            self.chain_spec().is_spurious_dragon_active_at_block(block_number);
        self.evm.db_mut().set_state_clear_flag(state_clear_flag);

        // NOTE: KubChain does NOT have:
        // - EIP-2935 block hash contract calls (no Shanghai)
        // - EIP-4788 beacon root contract calls (no beacon chain)

        // TODO: Apply hardfork state migrations if this is an activation block.
        // - Lausanne: Deploy V2 contract bytecodes + update storage slots
        // - Basel: Deploy V3 contract bytecodes + init SuperNode
        // These require the actual contract bytecodes from bkc source.
        if self
            .chain_spec()
            .inner()
            .fork(KubHardfork::Lausanne)
            .transitions_at_block(block_number)
        {
            tracing::info!(block_number, "Applying Lausanne hardfork state migration");
            // apply_lausanne_migration(&mut self.evm, self.chain_spec)?;
        }

        if self
            .chain_spec()
            .inner()
            .fork(KubHardfork::Basel)
            .transitions_at_block(block_number)
        {
            tracing::info!(block_number, "Applying Basel hardfork state migration");
            // apply_basel_migration(&mut self.evm, self.chain_spec)?;
        }

        Ok(())
    }

    fn execute_transaction_with_commit_condition(
        &mut self,
        tx: impl ExecutableTx<Self>,
        f: impl FnOnce(&ExecutionResult<<Self::Evm as Evm>::HaltReason>) -> CommitChanges,
    ) -> Result<Option<u64>, BlockExecutionError> {
        // Check gas limit against remaining block gas.
        let block_available_gas = self.evm.block().gas_limit - self.gas_used;

        if tx.tx().gas_limit() > block_available_gas {
            return Err(BlockValidationError::TransactionGasLimitMoreThanAvailableBlockGas {
                transaction_gas_limit: tx.tx().gas_limit(),
                block_available_gas,
            }
            .into());
        }

        // Execute transaction. Fees go to block_env.beneficiary (which is
        // set to SystemAddress when Chaophraya is active).
        let ResultAndState { result, state } = self
            .evm
            .transact(&tx)
            .map_err(|err| BlockExecutionError::evm(err, tx.tx().trie_hash()))?;

        if !f(&result).should_commit() {
            return Ok(None);
        }

        let gas_used = result.gas_used();
        self.gas_used += gas_used;

        // Build receipt.
        let receipt = Receipt {
            tx_type: tx.tx().tx_type(),
            success: result.is_success(),
            cumulative_gas_used: self.gas_used,
            logs: result.into_logs(),
        };
        self.receipts.push(receipt);

        // Commit state changes.
        self.evm.db_mut().commit(state);

        Ok(Some(gas_used))
    }

    fn finish(
        self,
    ) -> Result<(Self::Evm, BlockExecutionResult<Receipt>), BlockExecutionError> {
        let block_number: u64 = self.evm.block().number.saturating_to();
        let span = self.chain_spec().span();

        // === Post-execution system transactions ===
        //
        // In bkc, these are injected during Finalize(). In reth, they
        // happen in finish() after all user transactions have executed.

        // 1. commitSpan() — at span commitment blocks (span/2 + 1)
        if is_span_commitment_block(span, block_number) {
            tracing::debug!(
                block_number,
                span,
                "Span commitment block — commitSpan() system tx would be injected"
            );
            // TODO: Build and execute commitSpan system transaction.
            // This requires:
            // - Fetching eligible validators from ValidatorSet contract
            // - RLP-encoding the validator bytes
            // - Executing the system tx via the EVM
        }

        // 2. slash() — when out-of-turn block signed by official/super node
        let difficulty: u64 = self.evm.block().difficulty.saturating_to();
        if difficulty == DIFF_NO_TURN {
            // Check if signer is an official/super node and hasn't been slashed yet.
            // TODO: Query SlashManager.isSignerSlashed() and conditionally execute slash().
            tracing::debug!(
                block_number,
                signer = ?self.ctx.signer,
                "Out-of-turn block — slash check needed"
            );
        }

        // 3. distributeReward() — every block (transfers accumulated fees)
        //
        // In bkc: balance at SystemAddress is moved to coinbase, then
        // distributeReward() is called on StakeManager with msg.value = balance.
        //
        // TODO: Implement the actual balance transfer + system tx execution.
        tracing::debug!(
            block_number,
            "distributeReward() system tx would be injected"
        );

        Ok((
            self.evm,
            BlockExecutionResult {
                receipts: self.receipts,
                requests: Default::default(), // KubChain has no EIP-7685 requests
                gas_used: self.gas_used,
            },
        ))
    }

    fn set_state_hook(&mut self, hook: Option<Box<dyn OnStateHook>>) {
        self._state_hook = hook;
    }

    fn evm_mut(&mut self) -> &mut Self::Evm {
        &mut self.evm
    }

    fn evm(&self) -> &Self::Evm {
        &self.evm
    }
}

/// KubChain block executor factory.
///
/// Creates [`KubBlockExecutor`] instances with KubChain-specific execution
/// context. Uses the standard [`EthEvmFactory`](alloy_evm::EthEvmFactory) for
/// EVM creation since KubChain's EVM opcodes are standard Ethereum (capped at
/// London).
#[derive(Debug, Clone)]
pub struct KubBlockExecutorFactory<EvmF = alloy_evm::EthEvmFactory> {
    /// Chain specification.
    chain_spec: Arc<KubChainSpec>,
    /// EVM factory.
    evm_factory: EvmF,
}

impl<EvmF> KubBlockExecutorFactory<EvmF> {
    /// Creates a new [`KubBlockExecutorFactory`].
    pub fn new(chain_spec: Arc<KubChainSpec>, evm_factory: EvmF) -> Self {
        Self { chain_spec, evm_factory }
    }

    /// Returns the chain spec.
    pub fn chain_spec(&self) -> &Arc<KubChainSpec> {
        &self.chain_spec
    }
}

impl<EvmF> BlockExecutorFactory for KubBlockExecutorFactory<EvmF>
where
    EvmF: EvmFactory<
        Tx: TransactionEnv
                + FromRecoveredTx<TransactionSigned>
                + FromTxWithEncoded<TransactionSigned>,
        Spec = revm::primitives::hardfork::SpecId,
        Precompiles = PrecompilesMap,
    >,
    Self: 'static,
{
    type EvmFactory = EvmF;
    type ExecutionCtx<'a> = KubBlockExecutionCtx;
    type Transaction = TransactionSigned;
    type Receipt = Receipt;

    fn evm_factory(&self) -> &Self::EvmFactory {
        &self.evm_factory
    }

    fn create_executor<'a, DB, I>(
        &'a self,
        evm: EvmF::Evm<&'a mut State<DB>, I>,
        ctx: Self::ExecutionCtx<'a>,
    ) -> impl BlockExecutorFor<'a, Self, DB, I>
    where
        DB: Database + 'a,
        I: Inspector<EvmF::Context<&'a mut State<DB>>> + 'a,
    {
        KubBlockExecutor::new(evm, ctx)
    }
}

/// Checks whether a given address is a known official or super node.
///
/// Used to determine whether a slash system transaction should be injected
/// for out-of-turn blocks. Currently returns `false` as a placeholder.
///
/// TODO: Implement by checking the current snapshot's system contracts.
pub fn is_official_or_super_node(
    _signer: &Address,
    _system_contracts: &SystemContracts,
) -> bool {
    // Placeholder: requires querying the OfficialNode/SuperNode contract
    false
}
