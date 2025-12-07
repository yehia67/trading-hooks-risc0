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

2. **Deploy the ComplianceHook contract** using the Foundry script:

   ```bash
   forge script contracts/scripts/Deploy.s.sol:Deploy \
     --rpc-url http://localhost:8545 \
     --broadcast -vv
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




## Ethereum Sepolia Deployment

This repo also includes an end-to-end configuration for running the compliance hook against
the **Boundless** market on **Ethereum Sepolia**.

### 1. Configure environment variables

Use [`.env.example`](./.env.example) as a template and copy it to a local `.env` file:

```bash
cp .env.example .env
```

Then edit `.env` and fill in:

- `RPC_URL` – Sepolia RPC endpoint (for example `https://ethereum-sepolia-rpc.publicnode.com`).
- `PRIVATE_KEY` – EOA used to deploy and call the contracts.
- `BOUNDLESS_MARKET_ADDRESS` – Boundless market address for Ethereum Sepolia.
- `VERIFIER_ROUTER_ADDRESS` – `RiscZeroVerifierRouter` address on Ethereum Sepolia.
- `SET_VERIFIER_ADDRESS` – `SetVerifier` contract address on Ethereum Sepolia.
- `COMPLIANCE_HOOK_ADDRESS` – address of the `ComplianceHook` you deployed on Sepolia.
- `PINATA_JWT` – Pinata JWT token, if you use Pinata as the storage provider for guest programs.
- `AMOUNT`, `USER`, `PRODUCT_ID`, `KYC_PASSED`, `AML_PASSED` – example trade and compliance inputs.

See [`.env.example`](./.env.example) for concrete values and formatting.

### 2. Deploy ComplianceHook to Sepolia

First, deploy the `ComplianceHook` contract using the Foundry script. Make sure
`PRIVATE_KEY` and `VERIFIER_ROUTER_ADDRESS` are set in your environment (for example by
sourcing `.env`):

```bash
forge script contracts/scripts/Deploy.s.sol:Deploy \
  --fork-url "${RPC_URL}" \
  --broadcast -vvvv
```

The script will log the deployed `ComplianceHook` address. Copy that address into
`COMPLIANCE_HOOK_ADDRESS` in your `.env`.

### 3. Request a proof and call the hook on Sepolia

With `.env` configured, build and run the host app:

```bash
RUST_LOG=debug cargo run -p app --release
```

This will:

- Build the RISC Zero guest (`COMPLIANCE_ELF`).
- Use Boundless to submit a proof request to the Sepolia Boundless market.
- Wait for the request to be fulfilled.
- Call `ComplianceHook.beforeTrade` on Sepolia with the resulting `(journal, seal)`.

On success, you should see logs similar to:

```text
om/}: alloy_transport_http::reqwest_transport: retrieved response body. Use `trace` for full body bytes=46
2025-12-07T09:00:43.512848Z DEBUG alloy_provider::blocks: fetching block number=9787622
2025-12-07T09:00:43.512868Z DEBUG alloy_rpc_client::call: sending request method=eth_getBlockByNumber id=94
2025-12-07T09:00:43.513012Z DEBUG ReqwestTransport{url=https://ethereum-sepolia-rpc.publicnode.com/}: hyper_util::client::legacy::pool: reuse idle connection for ("https", ethereum-sepolia-rpc.publicnode.com)
2025-12-07T09:00:43.954447Z DEBUG ReqwestTransport{url=https://ethereum-sepolia-rpc.publicnode.com/}: alloy_transport_http::reqwest_transport: received response from server status=200 OK
2025-12-07T09:00:43.955625Z DEBUG hyper_util::client::legacy::pool: pooling idle connection for ("https", ethereum-sepolia-rpc.publicnode.com)
2025-12-07T09:00:43.955665Z DEBUG ReqwestTransport{url=https://ethereum-sepolia-rpc.publicnode.com/}: alloy_transport_http::reqwest_transport: retrieved response body. Use `trace` for full body bytes=8554
2025-12-07T09:00:43.955878Z DEBUG alloy_provider::blocks: yielding block number=9787622
2025-12-07T09:00:43.955898Z DEBUG alloy_provider::heart: handling block block_height=9787622
2025-12-07T09:00:43.955948Z DEBUG alloy_provider::heart: notifying tx=0xbeb6111a9be8e3188d1e79c6dc946db53d24acbee21c50fc91f18a9e212e4e86
2025-12-07T09:00:43.956090Z  INFO app: Tx 0xbeb6111a9be8e3188d1e79c6dc946db53d24acbee21c50fc91f18a9e212e4e86 confirmed
