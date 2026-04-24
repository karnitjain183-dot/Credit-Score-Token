# 🏦 CreditScore Credential — Soroban Smart Contract

> A portable, on-chain credit score credential system built on the **Stellar** blockchain using **Soroban** smart contracts.

---

## 📋 Project Description

**CreditScore Credential** is a decentralised credentialing protocol that lets trusted financial institutions issue, update, and revoke tamper-proof credit scores directly on the Stellar blockchain. Instead of siloed scores locked inside a single bureau's database, each credential lives on-chain — owned by the subject, verifiable by anyone, and fully portable across DeFi protocols, lending platforms, and real-world financial products.

The contract is written in **Rust** targeting the **Soroban** VM and follows the W3C Verifiable Credential spirit: the *issuer* attests to the *subject*, and third-party *verifiers* can check the result without ever contacting the original issuer.

---

## 🔍 What It Does

| Actor | Action |
|-------|--------|
| **Admin** | Deploys the contract, manages the whitelist of authorised issuers, can transfer admin rights |
| **Issuer** | Issues a credit-score credential to any Stellar address, updates the score over time, or revokes it |
| **Subject** | Holds a credential tied to their wallet; controls nothing directly but is the data owner |
| **Verifier** | Any on-chain or off-chain party that calls `verify_score` or `get_tier` to gate access |

### Core flow

```
Admin deploys → Admin adds Issuer → Issuer issues Credential → 
Anyone verifies → Issuer updates score → Issuer/Admin revokes
```

A `VERIFIED` event is emitted each time a score check occurs, enabling DeFi protocols to react on-chain (e.g. adjust collateral ratios) without ever reading the raw numeric score.

---

## ✨ Features

### 🔐 Role-Based Access Control
- Single **admin** address controls the issuer whitelist
- Only **whitelisted issuers** (banks, credit bureaus, DeFi protocols) can create or update credentials
- Admin rights are transferable via `transfer_admin`

### 📊 Score Tiers
Raw scores (300 – 850) are automatically mapped to human-readable tiers:

| Tier | Score Range |
|------|-------------|
| 🔴 Poor | 300 – 579 |
| 🟠 Fair | 580 – 669 |
| 🟡 Good | 670 – 739 |
| 🟢 Very Good | 740 – 799 |
| ⭐ Exceptional | 800 – 850 |

### 📜 Full Credential Lifecycle
- **Issue** — mint a new credential with score, context tag, and issuer metadata
- **Update** — bump the score and increment a version counter (full audit trail via events)
- **Revoke** — deactivate a credential without deleting history; revocation is permanent
- **Verify** — binary pass/fail check against a minimum threshold (great for lending gates)
- **Get Tier** — privacy-preserving tier lookup that never exposes the raw score

### 🔒 Privacy-Preserving Verification
`verify_score` and `get_tier` let verifiers confirm creditworthiness **without** learning the exact score — a key building block for private DeFi integrations.

### 📡 On-Chain Events
Every state change emits a Soroban event:
- `ISSUED`   — new credential created
- `UPDATED`  — score changed
- `REVOKED`  — credential deactivated
- `VERIFIED` — score threshold check performed

Events make the contract indexable by any Stellar Horizon-compatible indexer.

### 🧪 Full Test Suite
Ships with unit tests covering the happy path, update flow, revocation, threshold verification (pass & fail), invalid-score rejection, and tier derivation — all using Soroban's native `testutils`.

---

## 🚀 Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Install the Stellar CLI
cargo install --locked stellar-cli --features opt
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled contract will be at:
```
target/wasm32-unknown-unknown/release/credit_score_credential.wasm
```

### Test

```bash
cargo test
```

### Deploy to Testnet

```bash
# Configure testnet identity
stellar keys generate alice --network testnet

# Deploy
stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/credit_score_credential.wasm \
  --source alice \
  --network testnet

# Initialise (replace CONTRACT_ID and ADMIN_ADDRESS)
stellar contract invoke \
 
  --source alice \
  --network testnet \
  -- initialize \
  --admin ADMIN_ADDRESS
```

---

## 📂 Project Structure

```
credit-score-credential/
├── Cargo.toml          # Rust package manifest & Soroban dependencies
└── src/
    └── lib.rs          # Contract: types, storage, logic, and tests
```

---

## 🛠 Contract Interface

```rust
// Admin
fn initialize(env, admin: Address)
fn add_issuer(env, issuer: Address)
fn remove_issuer(env, issuer: Address)
fn transfer_admin(env, new_admin: Address)
fn get_admin(env) -> Address

// Credential lifecycle
fn issue_credential(env, issuer, subject, score: u32, context: String) -> CreditCredential
fn update_score(env, issuer, subject, new_score: u32, new_context: String) -> CreditCredential
fn revoke_credential(env, caller, subject)

// Queries
fn get_credential(env, subject) -> CreditCredential
fn verify_score(env, subject, min_score: u32) -> bool
fn get_tier(env, subject) -> ScoreTier
fn is_issuer(env, issuer) -> bool
```

---

## 🔮 Potential Extensions

- **ZK-proof integration** — prove score range without revealing the value
- **Expiry timestamps** — auto-expire credentials after N ledgers
- **Multi-issuer aggregation** — composite score from multiple bureaus
- **Cross-contract hooks** — lending protocols subscribe to `VERIFIED` events
- **Score history log** — append-only ledger of every score change

---



---

> Built with ❤️ on [Stellar](https://stellar.org) · Powered by [Soroban](https://soroban.stellar.org)
> Wallet address:GBZMZED7UTB222OQ75HFEVDUNIUX23CDULAYQA2VURM53EP2OW4MRPGI
>
> contact address:CC4NHZACNZCY42UGLQEHTTL4JFP3GE5KNHB5U6MYNOHWSCFOQGZESJC5
>
> https://stellar.expert/explorer/testnet/contract/CC4NHZACNZCY42UGLQEHTTL4JFP3GE5KNHB5U6MYNOHWSCFOQGZESJC5
>
> <img width="1892" height="869" alt="image" src="https://github.com/user-attachments/assets/96cfc2f5-c8b7-4859-85c0-ae31231cbd55" />

