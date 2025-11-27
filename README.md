# RWA Trading Hooks with RISC Zero

This project explores how to build **compliance-aware trading hooks for real-world assets (RWA)**
using **RISC Zero zkVM proofs** together with **Steel**, the Boundless zk-coprocessor library
that lets guest programs access smart contract state via **Boundless**.
Compliance checks (KYC/AML, jurisdiction rules, product-specific policies) run off-chain in a zkVM
"rules engine" and only expose a minimal on-chain decision:

> Is this user allowed to trade this product under the current conditions? (allowed: `true`/`false`)

All sensitive user data (KYC, AML, attributes) remains **off-chain**, and only commitments and
high-level results are visible on-chain.

## Architecture

Conceptually, the system looks like this:

```text
┌───────────────┐     ┌───────────────┐     ┌───────────────┐
│  User Registry│────▶│  Rules Engine │◀─── │    Product    │
└───────┬───────┘     └───────────────┘     └───────┬───────┘
        │                     ▲                     │
        │                     │                     │
        │             ┌───────┴───────┐             │
        └────────────▶│     Rules     │◀────────────┘
                      └───────────────┘
                              ▲
                              │
                      ┌───────┴───────┐
                      │   Conditions  │
                      └───────────────┘
```

- **User Registry**: maps user addresses (or IDs) to commitments of their private attributes
  (e.g. Merkle roots for KYC/AML data).
- **Product Registry**: maps product IDs (e.g. GOLD_US, STOCK_XYZ) to policy commitments
  that encode who is allowed to trade the product.
- **Rules / Conditions**: off-chain semantics like "gold for US citizens only",
  "KYC level ≥ 2", "not sanctioned", etc., encoded as hashes/commitments.
- **Rules Engine**: implemented as a **RISC Zero guest program** plus an on-chain verifier
  and hook contract. The guest verifies Merkle proofs, applies the rules, and outputs a
  boolean `allowed` in its journal.

On-chain, a **trading hook** contract consumes `(journal, seal)` from the zkVM and reverts
if `allowed == false`.

## High-Level Flow

1. **Off-chain preparation**
   - User obtains KYC/AML credentials from a provider.
   - The provider (or wallet) maintains a Merkle tree over user attributes and publishes only
     the Merkle root to the **User Registry**.
   - Products (e.g. gold, stocks) have policies encoded similarly and published as roots in
     the **Product Registry**.

2. **Proving (RISC Zero guest)**
   - Host code prepares input for the guest (user ID, product ID, Merkle proofs, etc.).
   - The guest reconstructs the relevant registry state, verifies Merkle links, applies the
     compliance rules, and commits a journal containing (among other things):
     - `user` (or user ID)
     - `productId`
     - `allowed: bool`

3. **Verification & trading hook**
   - A zk proof (seal) and the journal are submitted to an on-chain **hook contract**.
   - The hook verifies the proof using a RISC Zero verifier and checks that `allowed == true`.
   - If verification fails or `allowed == false`, the trade is rejected.

The result is a **privacy-preserving compliance gate** for RWA trading.

## Dependencies

To work with this project you will typically need:

- [Rust]
- [Foundry]
- [RISC Zero]
- [Boundless]
- [Steel]

Depending on whether you use remote proving (e.g. Bonsai) or local proving with Docker,
you may also need additional tooling; see the deployment guide for details.

### Configuring Bonsai (optional)

***Note:*** *To request an API key [complete the form here](https://bonsai.xyz/apply).* 

With the Bonsai proving service, you can produce a [Groth16 SNARK proof] that is verifiable on-chain.
You can get started by setting the following environment variables with your API key and associated URL.

```bash
export BONSAI_API_KEY="YOUR_API_KEY" # see form linked above
export BONSAI_API_URL="BONSAI_API_URL" # provided with your api key
```

## Project Layout (high level)

The concrete modules and contract names may evolve, but the layout follows this pattern:

- **contracts/**
  - On-chain verifier and trading hook contracts.
  - Registry contracts for users/products and associated commitments.
- **methods/**
  - zkVM guest code (compliance rules engine) and build script that produce the program ELF
    and image ID for the RISC Zero verifier.
- **apps/**
  - Host applications that construct inputs, request proofs, and submit `(journal, seal)`
    to the trading hook contracts.

Refer to `deployment-guide.md` for a concrete end-to-end flow based on this structure.

## Build and Test

### Build Rust components

```bash
cargo build
```

### Run Rust tests (host + methods)

```bash
cargo test
```

### Build Solidity contracts

```bash
forge build
```

### Run Solidity tests

```bash
forge test -vvv
```

### End-to-end script (if present)

If the repository provides an end-to-end script:

```bash
./e2e-test.sh
```

## Local Devnet Quickstart (Anvil)

The exact contracts and parameters depend on your current iteration. A typical local flow is:

1. **Start a local devnet**:

   ```bash
   anvil
   ```

2. **In a new terminal, set environment variables** (see `deployment-guide.md` for concrete examples):

   ```bash
   export ETH_WALLET_PRIVATE_KEY=<anvil_private_key>
   export BONSAI_API_KEY="YOUR_API_KEY"      # optional if proving via Bonsai
   export BONSAI_API_URL="BONSAI_API_URL"   # optional if proving via Bonsai
   ```

3. **Build the project**:

   ```bash
   cargo build
   forge build
   ```

4. **Deploy the verifier, registries, and trading hook** using your Foundry scripts.

5. **Run the host app** to request a proof and submit it to the hook.

6. **Query contract state** (e.g. exposure, balances, or positions) using `cast` to confirm
   that compliant trades succeed and non-compliant trades revert.

## Deploy Your Application

For complete, step-by-step instructions on deploying to a local devnet or a public testnet
such as Sepolia, see the [deployment guide].

[Foundry]: https://getfoundry.sh/
[Groth16 SNARK proof]: https://www.risczero.com/news/on-chain-verification
[RISC Zero]: https://dev.risczero.com/api/zkvm/install
[Boundless]: https://docs.boundless.network/developers/quick-start
[Sepolia]: https://www.alchemy.com/overviews/sepolia-testnet
[deployment guide]: ./deployment-guide.md
[Rust]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[Steel]: https://docs.boundless.network/developers/steel/quick-start
