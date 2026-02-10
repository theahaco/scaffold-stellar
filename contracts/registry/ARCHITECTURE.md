# Registry Contract Architecture

This document describes the architecture and design decisions of the Stellar Registry contract.

## Overview

The Registry contract is an on-chain package manager for Soroban smart contracts. It provides:

1. **Wasm Publishing** - Store contract bytecode with semantic versioning
2. **Contract Deployment** - Deploy contracts from published wasm with optional name registration
3. **Name Resolution** - Look up deployed contracts by human-readable names
4. **Upgrade Management** - Upgrade deployed contracts to new versions

## Contract Hierarchy

```mermaid
graph TD
    A[Root Registry] -->|deploys| B[Unverified Registry]
    A -->|registers| C["'registry' name"]
    A -->|registers| D["'unverified' name"]

    B -->|self-governed| B

    E[Users] -->|publish with manager approval| A
    F[Users] -->|publish freely| B
```

The root registry is a **managed** registry that requires manager approval for initial publishes. It automatically deploys an **unverified** registry where anyone can publish without approval.

## Storage Architecture

```mermaid
erDiagram
    INSTANCE_STORAGE {
        Symbol MANAGER "Optional manager address"
        Symbol ADMIN "Admin address (from admin-sep)"
    }

    PERSISTENT_STORAGE ||--o{ WASM_ENTRY : "WA:{name}"
    PERSISTENT_STORAGE ||--o{ CONTRACT_ENTRY : "CR:{name}"
    PERSISTENT_STORAGE ||--o{ HASH_ENTRY : "{hash}"

    WASM_ENTRY {
        Address author "Original publisher"
        String current_version "Latest version"
        Map versions "version -> hash"
    }

    CONTRACT_ENTRY {
        Address owner "Contract owner"
        Address contract "Deployed contract address"
    }

    HASH_ENTRY {
        unit value "Existence marker"
    }
```

### Storage Keys

| Prefix | Type | Purpose |
|--------|------|---------|
| `WA:{name}` | Persistent | Published wasm metadata |
| `CR:{name}` | Persistent | Deployed contract registry |
| `{hash}` | Persistent | Hash existence check (prevents duplicate publishes) |
| `MANAGER` | Instance | Manager address for approval workflow |

## Authorization Model

```mermaid
flowchart TD
    subgraph "Publish Flow"
        P1{First publish?}
        P1 -->|Yes| P2{Manager exists?}
        P1 -->|No| P3[Require author auth]
        P2 -->|Yes| P4[Require manager auth]
        P2 -->|No| P5[Require author auth]
    end

    subgraph "Deploy Flow"
        D1[deploy] --> D2{Manager exists?}
        D2 -->|Yes| D3[Require manager auth]
        D2 -->|No| D4[Require admin auth]

        D5[deploy_unnamed] --> D6[Require deployer auth only]
    end
```

### Key Authorization Rules

| Operation | Managed Registry | Unmanaged Registry |
|-----------|------------------|-------------------|
| Initial publish | Manager auth | Author auth |
| Subsequent publish | Author auth | Author auth |
| `deploy` | Manager auth | Admin auth |
| `deploy_unnamed` | Deployer auth | Deployer auth |
| `register_contract` | Manager auth | Owner auth |

## Core Operations

### Publishing

```mermaid
sequenceDiagram
    participant Author
    participant Manager
    participant Registry
    participant Ledger

    Author->>Registry: publish(name, wasm, version)

    alt First publish & managed
        Registry->>Manager: require_auth()
    else First publish & unmanaged
        Registry->>Author: require_auth()
    else Subsequent publish
        Registry->>Author: require_auth()
        Registry->>Registry: verify author matches
    end

    Registry->>Ledger: upload_contract_wasm(wasm)
    Ledger-->>Registry: hash

    Registry->>Registry: check hash not already published
    Registry->>Registry: validate version > current
    Registry->>Registry: store wasm entry
    Registry->>Registry: store hash marker

    Registry-->>Author: Ok(())
```

### Deployment Options

The registry provides two deployment methods with different use cases:

#### `deploy` - Named Deployment

```mermaid
sequenceDiagram
    participant User
    participant Manager
    participant Registry
    participant NewContract

    User->>Registry: deploy(wasm_name, contract_name, admin, init_args)

    alt Manager exists
        Registry->>Manager: require_auth()
    else No manager
        Registry->>User: require_auth()
    end

    Registry->>Registry: check contract_name not taken
    Registry->>Registry: fetch wasm hash
    Registry->>NewContract: deploy with hash(contract_name) as salt
    Registry->>Registry: register contract_name -> contract_id

    Registry-->>User: contract_id
```

**Use case**: When you want a human-readable name registered in the registry for discovery.

#### `deploy_unnamed` - Anonymous Deployment

```mermaid
sequenceDiagram
    participant Deployer
    participant Registry
    participant NewContract

    Deployer->>Registry: deploy_unnamed(wasm_name, salt, init_args)
    Deployer->>Deployer: require_auth()

    Registry->>Registry: fetch wasm hash
    Registry->>NewContract: deploy with provided salt

    Note over Registry: No name registered!

    Registry-->>Deployer: contract_id
```

**Use case**: When you just need to deploy a contract without registering a name.

**Why no manager auth?** The `deploy_unnamed` function intentionally skips manager authorization because:

1. **No name reservation**: It doesn't claim a name in the registry, so there's no namespace to protect
2. **Public wasm**: The wasm hash is already public - anyone can look it up and deploy it themselves using `env.deployer()`
3. **Deployer control**: The deployer provides their own salt and receives the contract, taking full responsibility
4. **Permissionless by design**: This enables use cases where users deploy their own instances of published contracts (e.g., token factories, personal vaults)

### Upgrade Flow

```mermaid
sequenceDiagram
    participant Owner
    participant Registry
    participant TargetContract

    Owner->>Registry: upgrade_contract(contract_name, wasm_name, version)

    Registry->>Registry: lookup contract by name
    Registry->>Registry: fetch new wasm hash

    alt Target has admin() method
        Registry->>TargetContract: admin()
        TargetContract-->>Registry: admin_address
        Registry->>Owner: require_auth() if owner == admin
    end

    Registry->>TargetContract: upgrade(new_hash)

    Registry-->>Owner: contract_id
```

## Name Normalization

Names are normalized to a canonical form before storage:

```mermaid
flowchart LR
    A[Input: "Hello_World"] --> B[Lowercase]
    B --> C["hello_world"]
    C --> D[Replace _ with -]
    D --> E["hello-world"]
    E --> F{Valid?}
    F -->|Yes| G[Store]
    F -->|No| H[Error]
```

### Validation Rules

1. Length: 1-64 characters
2. First character: ASCII alphabetic
3. Remaining: ASCII alphanumeric, `-`, or `_`
4. Not a Rust keyword (after normalization)

### Equivalent Names

These all resolve to the same canonical name `hello-world`:
- `hello_world`
- `hello-world`
- `Hello_World`
- `HELLO-WORLD`

## Version Management

Versions follow [Semantic Versioning](https://semver.org/):

```mermaid
flowchart TD
    A[New Version] --> B{Parse as semver}
    B -->|Invalid| C[Error: InvalidVersion]
    B -->|Valid| D{Compare to current}
    D -->|<=| E[Error: VersionMustBeGreaterThanCurrent]
    D -->|>| F[Accept]
```

### Ordering Examples

| Current | New | Result |
|---------|-----|--------|
| 1.0.0 | 1.0.1 | Accepted |
| 1.0.0 | 2.0.0-alpha | Accepted |
| 1.0.0 | 1.0.0 | Rejected |
| 1.0.1-alpha | 1.0.1 | Accepted |
| 1.0.1 | 1.0.1-beta | Rejected |

## TTL Management

The registry extends TTL for accessed entries:

```mermaid
flowchart LR
    A[fetch_hash] --> B[Extend wasm entry TTL]
    B --> C[Extend hash entry TTL]

    D[deploy] --> E[Extend wasm entry TTL]
    E --> F[Extend hash entry TTL]

    G[upgrade] --> H[Extend contract entry TTL]
```

Maximum TTL extension: **535,679 ledgers** (~31 days at 5s/ledger)

## Security Considerations

### Hash Uniqueness

Each wasm hash can only be published once across all names. This prevents:
- Hash squatting (publishing someone else's wasm under a different name)
- Confusion about which name is "official"

### Manager Trust Model

When a manager is set:
- Manager approves initial publishes (namespace control)
- Manager approves named deployments (prevents name squatting)
- Authors retain control of subsequent versions

### Contract Registration

`register_contract` allows registering externally-deployed contracts. The registry trusts that:
- On managed registries: Manager has verified the owner's claim
- On unmanaged registries: Owner self-attests (first-come-first-served)

## Public Interface

### Read Operations

| Method | Description |
|--------|-------------|
| `fetch_hash(name, version?)` | Get wasm hash for a published name |
| `current_version(name)` | Get latest version of published wasm |
| `fetch_contract_id(name)` | Get contract address by name |
| `fetch_contract_owner(name)` | Get owner of registered contract |
| `manager()` | Get current manager address |
| `admin()` | Get current admin address |

### Write Operations

| Method | Description |
|--------|-------------|
| `publish(name, author, wasm, version)` | Upload and register wasm |
| `publish_hash(name, author, hash, version)` | Register pre-uploaded wasm |
| `deploy(wasm_name, version?, contract_name, admin, init?, deployer?)` | Deploy and register |
| `deploy_unnamed(wasm_name, version?, init?, salt, deployer)` | Deploy without registration |
| `register_contract(name, address, owner)` | Register existing contract |
| `upgrade_contract(name, wasm_name, version?, fn?)` | Upgrade via registry |
| `dev_deploy(name, wasm, fn?)` | Upload and upgrade in one call |

### Admin Operations

| Method | Description |
|--------|-------------|
| `set_admin(new_admin)` | Transfer admin role |
| `set_manager(new_manager)` | Set new manager |
| `remove_manager()` | Remove manager (becomes unmanaged) |
| `upgrade(hash)` | Upgrade registry itself |
