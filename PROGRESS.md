# KubChain Migration Progress: bkc (geth) → kub-reth (reth)

> Last updated: 2026-03-01

## Overview

Migrating Bitkub Chain (KubChain) from `bkc` (legacy Go-Ethereum fork, circa 2021) to `kub-reth` (Rust-based reth fork). The chain runs PoSA consensus (Proof-of-Staked-Authority) built on Clique, without a beacon chain.

---

## Phase Status

| Phase | Description | Status |
|-------|-------------|--------|
| Phase 1 | Foundation (hardforks, chainspec, primitives) | ✅ Complete |
| Phase 2 | Consensus core (validation, snapshots, validator selection) | ✅ Complete |
| Phase 3 | System contracts (ABI bindings, contract caller) | ✅ Complete |
| Phase 4 | EVM execution, block executor, system tx injection | ✅ Structurally complete |
| Phase 5 | Node assembly + P2P configuration | ⬜ Not started |
| Phase 6 | Testing & verification | ⬜ Not started |

---

## Crate Structure

All 7 crates are registered in the workspace `Cargo.toml`:

```
crates/kubchain/
├── primitives/         # KubChain-specific types (Phase 1)
├── hardforks/          # KubHardfork enum (Phase 1)
├── chainspec/          # KubChainSpec (Phase 1)
├── consensus/          # KubConsensus — header/block validation (Phase 2)
├── contracts/          # ABI bindings + contract types (Phase 3)
├── evm/                # KubEvmConfig + block execution (Phase 4)
└── node/               # KubNode stub (Phase 5 — not yet implemented)
```

---

## Phase 1: Foundation ✅

### `kubchain-primitives` (`crates/kubchain/primitives/`)
- **`src/types.rs`**: Core constants and types
  - `SYSTEM_ADDRESS` — `0xffffFFFfFFffffffffffffffFfFFFfffFFFfFFfE` (fee accumulation address)
  - `DIFF_IN_TURN = 2`, `DIFF_NO_TURN = 1` — difficulty values for PoSA
  - `CHECKPOINT_INTERVAL = 1024`, `INMEMORY_SNAPSHOTS = 128`, `INMEMORY_SIGNATURES = 4096`
  - Extra-data constants: `EXTRA_VANITY = 32`, `EXTRA_SEAL = 65`, `EXTRA_CONTRACT_LEN = 20`, `EXTRA_NUM_CONTRACTS = 3`
  - `Validator { address, voting_power }` — full validator type
  - `MinimalVal { address, voting_power }` — RLP-encodable for commitSpan
  - `SystemContracts { stake_manager, slash_manager, super_node }` — contract addresses from snapshots
  - `SpanConfig { span, period, epoch }` — PoSA span configuration
  - `is_span_first_block(span, block_number)` and `is_span_commitment_block(span, block_number)` — span boundary helpers
- **`src/lib.rs`**: Re-exports all public types

### `kubchain-hardforks` (`crates/kubchain/hardforks/`)
- **`src/lib.rs`**: Uses reth's `hardfork!()` macro to define:
  ```rust
  KubHardfork {
      Erawan,             // Modified voting logic
      Chaophraya,         // PoSA activation (mainnet)
      ChaophrayaBangkok,  // PoSA activation (testnet variant)
      Lausanne,           // StakeManager V2, SlashManager V2
      Basel,              // SuperNode introduction, V3 contracts
  }
  ```
- **`src/dev.rs`**: Dev/test hardfork schedules (all forks at block 0)

### `kubchain-chainspec` (`crates/kubchain/chainspec/`)
- **`src/lib.rs`**: `KubChainSpec` wrapping reth's `ChainSpec` with:
  - `span_config: SpanConfig` — span/period/epoch
  - `validator_contract: Address` — BKCValidatorSet V1
  - `validator_contract_v2: Address` — BKCValidatorSet V2 (post-Lausanne)
  - Helper methods: `span()`, `period()`, `epoch()`
  - `Deref<Target = ChainSpec>` for transparent access to base chain spec
  - `inner()` returns the underlying `ChainSpec` for hardfork queries
- **`src/dev.rs`**: `kub_dev_chain_spec()` — dev chain with all forks at block 0, chain_id = 25925

---

## Phase 2: Consensus Core ✅

### `kubchain-consensus` (`crates/kubchain/consensus/`)
- **`src/lib.rs`**: `KubConsensus` struct with LRU caches for snapshots and signatures
- **`src/validation.rs`**: Header validation logic
  - Extra-data format validation (vanity + optional validators/contracts + seal)
  - Signer recovery from ECDSA signature in extra-data
  - Difficulty validation (must be DIFF_IN_TURN or DIFF_NO_TURN)
  - Timestamp validation against parent
  - Nonce validation (must be zero — not used in PoSA)
  - Uncle hash validation (must be empty)
- **`src/snapshot.rs`**: `KubSnapshot` — validator set snapshots
  - Fields: number, hash, validators (sorted), recents (recent signers), system_contracts
  - `apply()` method to update snapshot with new block headers
  - Recent signer tracking to prevent double-signing within span/2
  - Span boundary detection for validator set rotation
- **`src/validator.rs`**: Weighted random validator selection
  - Uses block hash from N blocks prior as seed
  - Binary search weighted random selection
  - Deterministic ordering per span
- **`src/extra_data.rs`**: Extra-data encoding/decoding
  - Standard block: `[32B vanity] [65B signature]`
  - Span boundary: `[32B vanity] [N×20B validator addrs] [60B system contracts] [65B signature]`
  - `decode_extra_data()`, `encode_extra_data()`, `recover_signer()`
- **`src/error.rs`**: `KubConsensusError` enum

---

## Phase 3: System Contracts ✅

### `kubchain-contracts` (`crates/kubchain/contracts/`)
- **`src/abi.rs`**: ABI bindings using `alloy-sol-types` sol!() macro
  - `IStakeManager` — `distributeReward(uint256 amount, address validator)`
  - `IValidatorSet` — `commitSpan(bytes validatorBytes)`, `getValidators()`, `getEligibleValidators()`
  - `IValidatorSetV2` — Extended with `getEligibleValidatorsV2()` (post-Lausanne)
  - `ISlashManager` — `slash(address signer, uint256 span)`, `isSignerSlashed(address, uint256)`
- **`src/types.rs`**: Contract result types
  - `ValidatorSetResult` — parsed result from getValidators/getEligibleValidators
  - `parse_validators()` — converts contract return data to `Vec<Validator>`
- **`src/lib.rs`**: Re-exports

---

## Phase 4: EVM Execution ✅ (Structural)

### `kubchain-evm` (`crates/kubchain/evm/`)

All files compile cleanly with `cargo check -p kubchain-evm`.

- **`src/config.rs`**: SpecId mapping **capped at LONDON**
  - `kub_revm_spec(chain_spec, header) -> SpecId` — maps header to revm SpecId
  - `kub_revm_spec_by_block_number(chain_spec, block_number) -> SpecId`
  - Mapping: Frontier → Homestead → Tangerine → SpuriousDragon → Byzantium → Constantinople → Petersburg → Istanbul → MuirGlacier → Berlin → **London** (max)
  - **No MERGE, SHANGHAI, CANCUN, PRAGUE** — KubChain never activates post-London specs

- **`src/execute.rs`**: Block executor factory and executor
  - `KubBlockExecutionCtx` — execution context with `Arc<KubChainSpec>` (no lifetime parameter to avoid coupling with ConfigureEvm trait)
    - `parent_hash: B256`
    - `chain_spec: Arc<KubChainSpec>`
    - `signer: Address` — block producer recovered from extra-data
    - `system_contracts: SystemContracts` — from current snapshot
  - `KubBlockExecutor<E>` implements `BlockExecutor` trait:
    - `apply_pre_execution_changes()`:
      - Sets state clear flag (Spurious Dragon)
      - Detects Lausanne/Basel hardfork activation blocks (TODO: actual state migrations)
    - `execute_transaction_with_commit_condition()`:
      - Gas limit check against remaining block gas
      - EVM execution via `self.evm.transact(&tx)`
      - Receipt building (tx_type, success, cumulative_gas_used, logs)
      - State commit via `self.evm.db_mut().commit(state)`
    - `finish()` — post-execution system tx injection:
      1. `commitSpan()` at span commitment blocks (`block_number % span == span/2 + 1`)
      2. `slash()` for out-of-turn blocks (`difficulty == DIFF_NO_TURN`) by official/super nodes
      3. `distributeReward()` every block (transfers accumulated fees from SystemAddress → StakeManager)
      - **All three are currently TODO placeholders** — structural hooks are in place
  - `KubBlockExecutorFactory<EvmF>` implements `BlockExecutorFactory`:
    - Creates `KubBlockExecutor` instances
    - Uses standard `EthEvmFactory` for EVM creation (opcodes are standard Ethereum)
  - `is_official_or_super_node()` — placeholder function (TODO: query OfficialNode/SuperNode contract)

- **`src/build.rs`**: Block assembler
  - `KubBlockAssembler` implements `BlockAssembler<F>` trait
  - Assembles KubChain-specific block headers:
    - Non-zero difficulty (preserved from PoSA)
    - No `withdrawals_root` (no Shanghai)
    - No `blob_gas_used`, `excess_blob_gas` (no EIP-4844)
    - No `parent_beacon_block_root` (no beacon chain)
    - No `requests_hash` (no EIP-7685)
  - Computes `receipts_root` and `logs_bloom` from execution receipts

- **`src/system_tx.rs`**: System transaction construction
  - `SYSTEM_TX_GAS = u64::MAX / 2` — effectively unlimited gas for system txs
  - `build_commit_span_calldata(validators)` — RLP-encodes validators, ABI-encodes `commitSpan(bytes)`
  - `build_slash_calldata(spoiled_validator, span_number)` — ABI-encodes `slash(address, uint256)`
  - `build_distribute_reward_calldata(amount, validator)` — ABI-encodes `distributeReward(uint256, address)`
  - `is_system_transaction(tx_gas_price, tx_sender, block_coinbase)` — identifies system txs (gas_price==0 && sender==coinbase)
  - Comprehensive unit tests for all builders

- **`src/lib.rs`**: Main KubEvmConfig
  - `KubNextBlockEnvCtx` — simple block attributes (timestamp, fee_recipient, gas_limit)
    - No beacon root, no withdrawals, no prev_randao (KubChain is not PoS merge)
  - `KubEvmConfig<EvmF>` implements `ConfigureEvm`:
    - `evm_env(header)`:
      - Sets `beneficiary = SYSTEM_ADDRESS` when Chaophraya hardfork is active
      - Otherwise uses `header.beneficiary`
      - Caps SpecId at LONDON
      - Sets `prevrandao = None`, `blob_excess_gas_and_price = None`
    - `next_evm_env(parent, attributes)`:
      - Same fee routing logic for next block
      - Calculates next base fee via `EthChainSpec::next_block_base_fee()`
    - `context_for_block(block)` and `context_for_next_block(parent, attributes)`:
      - Create `KubBlockExecutionCtx` with `Arc<KubChainSpec>`
      - `SystemContracts::default()` as placeholder (TODO: fetch from snapshot)
  - Tests verify: config creation with chain_id 25925, SystemAddress as beneficiary, spec capped at LONDON

---

## Phase 4 TODOs (Within Code)

These are marked as TODO in the code and don't block compilation, but need implementation for full functionality:

### 1. Hardfork State Migrations (`execute.rs:apply_pre_execution_changes`)
- **Lausanne migration**: Deploy V2 contract bytecodes + update storage slots for StakeManager V2, SlashManager V2
- **Basel migration**: Deploy V3 contract bytecodes + init SuperNode contract
- Requires extracting actual contract bytecodes from bkc Go source (`bkc/consensus/clique/hardfork/lausanne/instruction.go` and `bkc/consensus/clique/hardfork/basel/instruction.go`)

### 2. System Transaction Execution (`execute.rs:finish`)
- **commitSpan**: Fetch eligible validators from ValidatorSet contract, RLP-encode, execute system tx via EVM
- **slash**: Query `SlashManager.isSignerSlashed()`, conditionally execute `slash()` system tx
- **distributeReward**: Read SystemAddress balance, transfer to coinbase, call `StakeManager.distributeReward()` with msg.value
- Each requires constructing a proper `TransactionSigned` with gas_price=0, gas=SYSTEM_TX_GAS, and executing via the EVM

### 3. `is_official_or_super_node()` (`execute.rs`)
- Currently returns `false` (placeholder)
- Needs to query OfficialNode contract (pre-Basel) or SuperNode contract (post-Basel) from snapshot's system_contracts

### 4. SystemContracts from Snapshot (`lib.rs:context_for_block`)
- Currently uses `SystemContracts::default()` (all zero addresses)
- Should fetch from the consensus snapshot at the given block

---

## Phase 5: Node Assembly ⬜ (Next)

### What Needs to Be Done

#### 5.1 KubNode Type (`crates/kubchain/node/`)
Currently a stub (`src/lib.rs` has only TODO comments). Needs:

```rust
pub struct KubNode;

impl NodeTypes for KubNode {
    type Primitives = EthPrimitives;
    type ChainSpec = KubChainSpec;
    type Storage = EthStorage;
    type Payload = (); // or KubEngineTypes
}
```

Key components to wire via `ComponentsBuilder`:
- **Consensus**: `KubConsensus` (from Phase 2)
- **EVM**: `KubEvmConfig` (from Phase 4)
- **Pool**: `EthereumPoolBuilder` (reuse from reth — standard tx pool)
- **Network**: `EthereumNetworkBuilder` (reuse — ETH66 supported)
- **Payload**: Custom or stub (KubChain doesn't use engine API like PoS Ethereum)

#### 5.2 Binary Entry Point
Create `bin/kub-reth/src/main.rs`:
```rust
fn main() {
    Cli::<KubChainSpecParser>::parse().run(async move |builder, _| {
        let handle = builder.node(KubNode::default()).launch().await?;
        handle.node_exit_future.await
    })
}
```

Also need `KubChainSpecParser` to load KubChain genesis from CLI.

#### 5.3 P2P Configuration
- ETH66 protocol (confirmed compatible — reth supports ETH66 via `EthVersion::Eth66`)
- DiscV4 for peer discovery (same as bkc)
- KubChain boot nodes configuration
- Fork ID must match existing bkc nodes (same genesis hash + same hardfork block numbers)

### Reference Files
- `kub-reth/crates/ethereum/node/src/node.rs` — EthereumNode pattern to follow
- `kub-reth/bin/reth/src/main.rs` — entry point pattern
- `kub-reth/examples/bsc-p2p/src/chainspec.rs` — BSC custom chainspec pattern

---

## Phase 6: Testing & Verification ⬜

- Unit tests for each consensus component
- `cargo test -p kubchain-*` across all crates (needs GVM_ROOT env issue resolved)
- Sync test: connect kub-reth to existing bkc testnet
- Historical replay: replay blocks and compare state roots with bkc
- P2P interop: verify kub-reth peers with bkc nodes
- Fork ID compatibility test
- Hardfork transition tests (Chaophraya, Lausanne, Basel boundaries)

---

## Key Design Decisions

1. **SpecId capped at LONDON**: KubChain never activates post-London specs. No MERGE, SHANGHAI, CANCUN, PRAGUE.

2. **Fee routing via SystemAddress**: After Chaophraya hardfork, `block_env.beneficiary = SYSTEM_ADDRESS`. All gas fees accumulate there, then get distributed via `distributeReward()` system tx in `finish()`.

3. **`Arc<KubChainSpec>` in execution context**: `KubBlockExecutionCtx` uses `Arc<KubChainSpec>` instead of `&'a KubChainSpec` to avoid lifetime coupling with the `ConfigureEvm` trait's `context_for_block` method (which doesn't guarantee `&'a self`).

4. **No beacon chain integration**: KubChain runs as a single binary without consensus layer separation. No engine API, no beacon root, no withdrawals, no prev_randao.

5. **Reuse standard reth components**: EthPrimitives, EthStorage, EthEvmFactory (standard EVM opcodes), EthereumPoolBuilder, EthereumNetworkBuilder — only consensus, EVM config, and node assembly are custom.

---

## File Inventory

```
crates/kubchain/
├── primitives/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── types.rs
├── hardforks/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── dev.rs
├── chainspec/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── dev.rs
├── consensus/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       ├── extra_data.rs
│       ├── snapshot.rs
│       ├── validation.rs
│       └── validator.rs
├── contracts/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── abi.rs
│       └── types.rs
├── evm/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── build.rs
│       ├── config.rs
│       ├── execute.rs
│       └── system_tx.rs
└── node/
    ├── Cargo.toml
    └── src/
        └── lib.rs          # Stub — Phase 5
```

## Design Documents

Located in `docs/design/kubchain/`:
- `README.md` — Overview
- `migration-overview.md` — Full migration plan
- `consensus-design.md` — Consensus module design
- `crate-structure.md` — Crate organization
- `evm-compatibility.md` — EVM version compatibility
- `p2p-compatibility.md` — P2P protocol compatibility
- `bsc-architecture.md` — BSC architecture analysis for reuse
- `phase4-block-execution.md` — Phase 4 detailed design

## Plan File

The full migration plan is at: `.claude/plans/witty-squishing-kahn.md`

---

## Environment Notes

- **GVM_ROOT issue**: Running `cargo test` triggers a `GVM_ROOT not set` error from shell profile. Workaround: run cargo with a clean PATH that excludes the GVM initialization, or fix the shell profile.
- **Compilation**: `cargo check -p kubchain-evm` passes cleanly (no errors, no warnings) as of Phase 4 completion.
- **Rust version**: cargo 1.89.0
