# KubChain Consensus Design

## Overview

KubChain uses a Proof-of-Staked-Authority (PoSA) consensus, originally built on top of Go-Ethereum's Clique PoA engine. This document describes how to implement it in Rust as a pluggable module for reth.

## Architecture

```mermaid
graph TB
    subgraph "Reth Consensus Trait Hierarchy"
        HV["HeaderValidator&lt;Header&gt;<br/>validate_header()<br/>validate_header_against_parent()"]
        C["Consensus&lt;Block&gt;<br/>validate_body_against_header()<br/>validate_block_pre_execution()"]
        FC["FullConsensus&lt;NodePrimitives&gt;<br/>validate_block_post_execution()"]

        HV --> C
        C --> FC
    end

    subgraph "KubConsensus Implementation"
        KC["KubConsensus&lt;P&gt;"]
        SNAP["KubSnapshot"]
        SCC["SystemContractCaller"]
        VS["ValidatorSelector"]
        ED["ExtraDataCodec"]

        KC --> SNAP
        KC --> SCC
        KC --> VS
        KC --> ED
    end

    FC -.-> KC

    style HV fill:#4A90D9,color:white
    style C fill:#4A90D9,color:white
    style FC fill:#4A90D9,color:white
    style KC fill:#E8744F,color:white
```

## Go-to-Rust Mapping

### Core Struct

```
bkc: consensus/clique/clique.go → Clique struct
rust: crates/kubchain/consensus/src/lib.rs → KubConsensus<P>
```

```mermaid
classDiagram
    class KubConsensus {
        -chain_spec: Arc~KubChainSpec~
        -snapshot_cache: RwLock~LruCache~
        -signature_cache: RwLock~LruCache~
        -contract_caller: SystemContractCaller
        +validate_header(header)
        +validate_header_against_parent(header, parent)
        +validate_body_against_header(body, header)
        +validate_block_pre_execution(block)
        +validate_block_post_execution(block, result)
    }

    class KubSnapshot {
        +number: u64
        +hash: B256
        +signers: HashSet~Address~
        +validators: Vec~Address~
        +recents: HashMap~u64, Address~
        +system_contracts: SystemContracts
        +apply(header) KubSnapshot
        +store(db)
        +load(db, hash) KubSnapshot
    }

    class SystemContractCaller {
        -provider: P
        +distribute_reward(contract, amount, validator)
        +commit_span(validator_bytes)
        +slash(contract, signer, span)
        +is_slashed(contract, signer, span) bool
        +get_validators(header) Vec~Validator~
        +get_eligible_validators(header) Vec~Validator~
        +get_current_span(header) u64
    }

    class SystemContracts {
        +stake_manager: Address
        +slash_manager: Address
        +official_node: Address
        +super_node: Address
    }

    class Validator {
        +address: Address
        +voting_power: U256
    }

    KubConsensus --> KubSnapshot
    KubConsensus --> SystemContractCaller
    KubSnapshot --> SystemContracts
    SystemContractCaller --> Validator
```

## Block Lifecycle

```mermaid
sequenceDiagram
    participant Net as P2P Network
    participant Cons as KubConsensus
    participant Snap as KubSnapshot
    participant EVM as Block Executor
    participant SC as System Contracts
    participant State as State DB

    Net->>Cons: New block received

    Note over Cons: Phase 1: Header Validation
    Cons->>Cons: validate_header()
    Cons->>Cons: Recover signer from extra-data signature
    Cons->>Cons: Check difficulty (2=in-turn, 1=out-of-turn)
    Cons->>Snap: Load snapshot for parent block
    Snap-->>Cons: Validator set, system contracts
    Cons->>Cons: Verify signer is authorized validator
    Cons->>Cons: validate_header_against_parent()
    Cons->>Cons: Check timestamp, block number

    Note over Cons: Phase 2: Body Validation
    Cons->>Cons: validate_body_against_header()
    Cons->>Cons: Check tx root, uncle hash (empty)
    Cons->>Cons: validate_block_pre_execution()
    Cons->>Cons: Verify system tx validity

    Note over EVM: Phase 3: Block Execution
    EVM->>State: Execute user transactions
    EVM->>SC: distributeReward (system tx)

    alt Span commitment block (span/2 + 1)
        EVM->>SC: commitSpan (system tx)
    end

    alt Out-of-turn block by official/super node
        EVM->>SC: slash (system tx)
    end

    Note over Cons: Phase 4: Post-Execution
    Cons->>Cons: validate_block_post_execution()
    Cons->>Cons: Verify state root, receipt root
    Cons->>Snap: Update snapshot at checkpoint
```

## Validator Selection Algorithm

The weighted random validator selection (from `bkc/consensus/clique/clique.go:1390-1441`):

```mermaid
flowchart TD
    START["Start: selectNextValidatorSet()"]
    GET["Get eligible validators from contract<br/>(address + voting power)"]
    SEED["Compute seed from block hash<br/>5 blocks prior"]
    WEIGHTS["Build cumulative weight array<br/>weights[i] = sum(power[0..i])"]
    RAND["Generate random number in range<br/>[0, total_weight) using seed"]
    BSEARCH["Binary search for validator<br/>whose cumulative weight >= random"]
    SELECT["Selected validator added to set"]
    REMOVE["Remove selected from candidates"]
    DONE{All positions filled?}

    START --> GET --> SEED --> WEIGHTS --> RAND --> BSEARCH --> SELECT --> REMOVE --> DONE
    DONE -->|No| RAND
    DONE -->|Yes| END["Return ordered validator set"]
```

## Span Management

```mermaid
graph LR
    subgraph "Span N (e.g., 200 blocks)"
        B0["Block 0<br/>Span start<br/>Apply new validators"]
        B1["..."]
        B100["Block span/2+1<br/>Commit next span"]
        B2["..."]
        B199["Block span-1<br/>Last block"]
    end

    subgraph "Span N+1"
        NB0["Block 0<br/>New validators active"]
        NB1["..."]
    end

    B0 --> B1 --> B100 --> B2 --> B199 --> NB0 --> NB1

    style B0 fill:#4A90D9,color:white
    style B100 fill:#E8744F,color:white
    style NB0 fill:#4A90D9,color:white
```

**Key timing:**
- **Span first block** (`block_number % span == 0`): New validator set becomes active
- **Span commitment block** (`block_number % span == span/2 + 1`): Commit next validator set via contract
- **Validator update check**: At blocks before span first block, encode validators in extra-data

## Extra-Data Format

```mermaid
graph TB
    subgraph "Standard Block Extra-Data"
        V1["Vanity<br/>32 bytes"]
        S1["ECDSA Signature<br/>65 bytes"]
        V1 --> S1
    end

    subgraph "Span Boundary Extra-Data"
        V2["Vanity<br/>32 bytes"]
        VAL["Validator Addresses<br/>N x 20 bytes"]
        SC["System Contracts<br/>3 x 20 bytes<br/>(StakeManager + SlashManager + SuperNode)"]
        S2["ECDSA Signature<br/>65 bytes"]
        V2 --> VAL --> SC --> S2
    end
```

## Difficulty Rules

| Condition | Difficulty | Description |
|-----------|-----------|-------------|
| In-turn signer | 2 (`DIFF_IN_TURN`) | Authorized validator for this slot |
| Out-of-turn signer | 1 (`DIFF_NO_TURN`) | Fallback/super node signing |

Pre-PoSA (before Chaophraya): Standard Clique recent-signer check prevents validator spam.
Post-PoSA: Simplified with delay mechanics. SuperNode permitted out-of-turn without restriction.

## System Transactions

Zero-gas transactions injected during block finalization:

```mermaid
flowchart TD
    FIN["Block Finalization (FinalizeAndAssemble)"]

    HF{Hardfork<br/>activation block?}
    FIN --> HF
    HF -->|Lausanne| LAUS["Deploy V2 contracts<br/>Update storage slots"]
    HF -->|Basel| BAS["Deploy V3 contracts<br/>Initialize SuperNode"]
    HF -->|No| NEXT1
    LAUS --> NEXT1
    BAS --> NEXT1

    NEXT1["Check validator update needed"]
    VU{Span first<br/>block next?}
    NEXT1 --> VU
    VU -->|Yes| UPDATE["Encode validators + contracts<br/>in extra-data"]
    VU -->|No| NEXT2
    UPDATE --> NEXT2

    NEXT2["Check span commitment"]
    SC{Span commitment<br/>block?}
    NEXT2 --> SC
    SC -->|Yes| COMMIT["System TX: commitSpan()"]
    SC -->|No| NEXT3
    COMMIT --> NEXT3

    NEXT3["Check slashing"]
    SL{Out-of-turn block<br/>by super/official?}
    NEXT3 --> SL
    SL -->|Yes| SLASH["System TX: slash()"]
    SL -->|No| NEXT4
    SLASH --> NEXT4

    NEXT4["Distribute rewards"]
    DIST["System TX: distributeReward()<br/>Fees from SystemAddress → StakeManager"]
    NEXT4 --> DIST
```

## Snapshot Persistence

- **Checkpoint interval**: Every 1024 blocks
- **In-memory cache**: LRU with 128 entries
- **Signature cache**: LRU with 4096 entries for signer recovery
- **DB storage**: Persist snapshots to MDBX for durability
- **Snapshot fields**: Number, Hash, Signers, Validators, Recents, SystemContracts

## Source Reference

| Component | bkc File | Lines |
|-----------|----------|-------|
| Core consensus | `consensus/clique/clique.go` | All (1,619 lines) |
| Snapshot | `consensus/clique/snapshot.go` | All |
| Contract client | `consensus/clique/contract/client.go` | All |
| ABI definitions | `consensus/clique/contract/abi.go` | All |
| Contract interface | `consensus/clique/contract_client.go` | 18-87 |
| SystemContracts type | `consensus/clique/ctypes/common.go` | 15-20 |
| Validator selection | `consensus/clique/clique.go` | 1390-1441 |
| Block finalization | `consensus/clique/clique.go` | 817-938 |
| Slashing | `consensus/clique/clique.go` | 1115-1141 |
| Reward distribution | `consensus/clique/clique.go` | 1143-1155 |
| Span commitment | `consensus/clique/clique.go` | 1157-1176 |
