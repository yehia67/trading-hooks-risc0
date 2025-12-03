## Host Program tutorial
This tutorial explains [main.rs](../apps/src/main.rs) line by line and map each part to the concepts from the Boundless “Tracking” tutorial (client setup, request creation, submission, fulfillment, and consumption). I’ll first pull in the tracking tutorial so I can reference its terminology precisely, then I’ll give you a structured explanation.

I’ll walk through the file top‑to‑bottom and, for each major block, I’ll connect it directly to concepts and sections from the Boundless “Tracking Proof Requests” tutorial (especially the “Programmatically → Basic Request Tracking / How It Works” parts). After that I’ll give you a short mental model of the whole flow.

I’ll go top‑to‑bottom through [main.rs](cci:7://file:///Users/yehiatarek/Documents/projects/solidity/rwa-trading-hooks-risc0/apps/src/main.rs:0:0-0:0) and at each step I’ll relate it to the concepts in the **“Tracking Proof Requests”** tutorial (especially *Programmatically → Basic Request Tracking* and *How It Works*).

I’ll assume you have the file open as reference:

[apps/src/main.rs](cci:7://file:///Users/yehiatarek/Documents/projects/solidity/rwa-trading-hooks-risc0/apps/src/main.rs:0:0-0:0)  
@[/Users/yehiatarek/Documents/projects/solidity/rwa-trading-hooks-risc0/apps/src/main.rs:L1-L154]

---

## 1. High‑level mapping to the Tracking tutorial

The tutorial’s **programmatic tracking** flow is roughly:

1. Configure a **client** (RPC, deployment, auth, storage).
2. Construct a **request** (program + input).
3. **Submit** it to the Boundless market.
4. **Track** it by `request_id` until *fulfilled* (or failed / expired).
5. Use the fulfillment’s **seal** (+ journal hash) in your app / contracts.

Your [main.rs](cci:7://file:///Users/yehiatarek/Documents/projects/solidity/rwa-trading-hooks-risc0/apps/src/main.rs:0:0-0:0) mirrors that exactly:

- Lines ~72–94: **client setup**.
- Lines 96–112: **request construction** (`new_request`, program + stdin).
- Line 114: **`submit_onchain`** → returns `request_id`, `expires_at`.
- Lines 116–120: **`wait_for_request_fulfillment`** → request tracking.
- Lines 122–135: consume `fulfillment.seal` + journal in **ComplianceHook.beforeTrade`.

---

## 2. Imports and constants (context & tracking prerequisites)

```rust
use crate::compliance_hook::IComplianceHook::IComplianceHookInstance;
use boundless_market::{Client, Deployment, StorageProviderConfig};
use guests::COMPLIANCE_ELF;
...
pub const TX_TIMEOUT: Duration = Duration::from_secs(30);

mod compliance_hook {
    alloy::sol!( ... "../contracts/src/IComplianceHook.sol" );
}
```

Tutorial alignment:

- The tutorial assumes you’re using a **Boundless client SDK**.  
  Here that’s the `boundless_market::Client`.
- `Deployment` and `StorageProviderConfig` line up with the tutorial’s description of:
  - choosing which **market deployment** you talk to (testnet/mainnet),
  - and configuring where proofs/journals are **stored** (S3, etc.).
- `COMPLIANCE_ELF` is your **guest program binary** (the “program” in *request = program + input*).
- `IComplianceHookInstance` is just a typed EVM contract binding; not part of Boundless itself but it’s the **consumer** of the Boundless proof+journal.

---

## 3. CLI arguments (how you supply the tracking inputs)

```rust
struct Args {
    amount: u32,
    rpc_url: Url,
    private_key: PrivateKeySigner,
    compliance_hook_address: Address,
    user: Address,
    product_id: B256,
    kyc_passed: bool,
    aml_passed: bool,
    program_url: Option<Url>,
    offchain: bool,
    storage_config: StorageProviderConfig,
    deployment: Option<Deployment>,
}
```

Mapping to the tutorial:

- The tutorial’s examples show you must provide:
  - **RPC endpoint** (`rpc_url`),
  - **authentication / signer** (`private_key`),
  - **deployment and storage config**.
- You additionally provide **business inputs**:
  - `user`, `product_id`, `amount`, and the off‑chain flags `kyc_passed`, `aml_passed`.
- `program_url` corresponds to the tutorial’s option to **run a remote program** instead of an embedded ELF.
- `offchain` is present but not used yet in this file; you can ignore it for now.

---

## 4. [main](cci:1://file:///Users/yehiatarek/Documents/projects/solidity/rwa-trading-hooks-risc0/apps/src/main.rs:73:0-152:1) bootstrap (logging + env) – standard, not Boundless‑specific

```rust
tracing_subscriber::fmt()
    .with_env_filter(...)
    .init();

match dotenvy::dotenv() { ... }

let args = Args::parse();
```

- This is just **logging** and **config loading**.  
  In the tutorial, they assume you have a way to get your credentials / URLs; here it’s via env + CLI flags.

---

## 5. Building the Boundless client (tutorial: “Programmatically → Basic Request Tracking – client setup”)

```rust
let client = Client::builder()
    .with_rpc_url(args.rpc_url)
    .with_deployment(args.deployment)
    .with_storage_provider_config(&args.storage_config)?
    .with_private_key(args.private_key)
    .build()
    .await
    .context("failed to build boundless client")?;
```

Direct mapping to the tutorial:

- **`with_rpc_url`**: same as setting the RPC endpoint in the *Basic Request Tracking* examples.
- **`with_deployment`**:
  - Tutorial talks about different **market deployments** (e.g. testnets).
  - You’re passing `deployment` from CLI/env so you can target the appropriate one.
- **`with_storage_provider_config`**:
  - Tutorial’s *How It Works* explains that proofs/journals may be stored with a configured storage provider.
  - This struct is how you wire that in.
- **`with_private_key`**:
  - Equivalent to configuring **authentication / signer** in the programmatic examples.
- `.build().await`:
  - Corresponds to “Create the client instance you’ll use to submit and track requests.”

So after this, you’re at the same place as the tutorial right before they construct a `request`.

---

## 6. Preparing the guest input (tutorial: “program + input”)

```rust
tracing::info!("Attempting trade with amount: {}", args.amount);
type Input = (Address, B256, bool, bool);
let input = (args.user, args.product_id, args.kyc_passed, args.aml_passed);
let input_bytes = <Input>::abi_encode(&input);
```

Tutorial concept:

- The tutorial says each **request** is effectively:
  - the **program** you want to run (your compliance guest),
  - plus the **input** serialized as bytes (often ABI‑encoded).
- Here:
  - `Input` is the ABI type `(user, product_id, kyc_passed, aml_passed)`.
  - `input_bytes` is the **encoded stdin** you’ll pass to the guest.

This matches the tracking examples where they talk about “passing input data” into the program.

---

## 7. Building the request (tutorial: “create a request” / “How It Works”)

```rust
let request = if let Some(program_url) = args.program_url {
    client
        .new_request()
        .with_program_url(program_url)?
        .with_stdin(input_bytes.clone())
} else {
    client
        .new_request()
        .with_program(COMPLIANCE_ELF)
        .with_stdin(input_bytes)
};
```

Tutorial mapping:

- `client.new_request()` = the tutorial’s step “construct a **request object**”.
- Two ways to specify the **program**:
  - `with_program_url(program_url)` → tutorial’s pattern of using a **remote artifact**.
  - `with_program(COMPLIANCE_ELF)` → tutorial’s pattern of using an **embedded binary**.
- `with_stdin(input_bytes)`:
  - This is exactly their “set the input payload for the program” step.

By the end of this block you have a **fully‑formed Boundless request**: program + input + client config.

---

## 8. Submitting the request (tutorial: “submit the request”)  

```rust
let (request_id, expires_at) = client.submit_onchain(request).await?;
```

Tutorial references:

- In *Programmatically → Basic Request Tracking*, they show you call a method to submit a request and get back:
  - a **`request_id`** you can use to track it,
  - and sometimes metadata like **expiration times**.
- `submit_onchain`:
  - Publishes the request to the Boundless market on‑chain.
  - Returns `request_id` and `expires_at`.
- `expires_at` aligns with the “How It Works” section where they explain request lifecycle and **expiry**.

At this point you’re at the same stage as the tutorial’s “request has been created; now you can track it.”

---

## 9. Tracking the request until fulfillment (tutorial: “Basic Request Tracking”, “Request Status Types”)

```rust
tracing::info!("Waiting for request {:x} to be fulfilled", request_id);
let fulfillment = client
    .wait_for_request_fulfillment(request_id, Duration::from_secs(5), expires_at)
    .await?;
tracing::info!("Request {:x} fulfilled", request_id);
```

Tutorial alignment:

- They describe tracking a request by:
  - periodically checking its status (queued, running, fulfilled, failed, etc.), or
  - using a helper that **waits** until it’s fulfilled.
- `wait_for_request_fulfillment` is essentially the second style:
  - It hides the polling, using a **poll interval** (`Duration::from_secs(5)`).
  - It respects `expires_at` to stop once the request can’t be fulfilled anymore.
- The `fulfillment` you get back corresponds to what the tutorial calls the **fulfillment record**:
  - It contains the **seal** (proof),
  - and the ability to retrieve the **journal** or verify it.

Your code later uses `fulfillment.seal` when calling the hook.

---

## 10. Building the journal for the contract (tutorial: “How It Works” – journal + seal consume)

```rust
let allowed = args.kyc_passed && args.aml_passed;
type Output = (Address, B256, bool);
let journal_bytes = <Output>::abi_encode(&(args.user, args.product_id, allowed));
let journal = Bytes::from(journal_bytes);
```

Tutorial concept:

- In *How It Works*, they explain that:
  - The guest emits a **journal**,
  - The proof/fulfillment ties that journal to the program and input.
- Here you mirror the same **journal structure** that your guest emits:
  - `(user, product_id, allowed)`.
- In practice, the RISC Zero guest computed this same tuple and committed it.  
  The Solidity hook only sees the **journal bytes** + **seal**; it never sees KYC/AML flags.

This is the “consumer side” the tutorial talks about: **you use the seal + journal hash to verify a computation result elsewhere**.

---

## 11. Calling the on‑chain ComplianceHook (consumer using seal + journal)

```rust
let hook =
    IComplianceHookInstance::new(args.compliance_hook_address, client.provider().clone());
let call_before_trade = hook
    .beforeTrade(
        args.user,
        args.product_id,
        U256::from(args.amount),
        journal,
        fulfillment.seal,
    )
    .from(client.caller());
```

Tutorial relationship:

- The tracking tutorial focuses on *tracking* and says that other systems (contracts, services) can then:
  - take the **seal** and **journal**,
  - and **verify** / act on them.
- Your `ComplianceHook.beforeTrade` is exactly that consumer:
  - It calls `VERIFIER.verify(seal, IMAGE_ID, sha256(journal))` (in Solidity) to check the proof.
  - It decodes the journal and ensures `allowed == true` and that `(user, productId)` match.
- The Rust side is just preparing the transaction that delivers:
  - `journal` and
  - `fulfillment.seal`
  to the on‑chain verifier.

So this is the “downstream use” of what the tutorial produced via tracking.

---

## 12. Broadcasting and confirming the tx (standard EVM flow)

```rust
let pending_tx = call_before_trade
    .send()
    .await
    .context("failed to broadcast tx")?;
tracing::info!("Broadcasting tx {}", pending_tx.tx_hash());
let tx_hash = pending_tx
    .with_timeout(Some(TX_TIMEOUT))
    .watch()
    .await
    .context("failed to confirm tx")?;
tracing::info!("Tx {:?} confirmed", tx_hash);
```

- This is classic **transaction lifecycle**:
  - `send()` → broadcast.
  - `watch()` with `TX_TIMEOUT` → wait for confirmation.
- The tracking tutorial doesn’t go deep into tx broadcasting; it assumes you already know how to send transactions with your signer/provider.  
  Your code is just doing that using the Boundless client’s provider + caller.

---

## 13. Mental model: connecting it all to the Tracking tutorial

Putting it in the tutorial’s language:

1. **Configure client**  
   `Client::builder().with_rpc_url(...).with_deployment(...).with_storage_provider_config(...).with_private_key(...)`.

2. **Create a request**  
   `client.new_request()` + `with_program(COMPLIANCE_ELF)` / `with_program_url` + `with_stdin(input_bytes)`.

3. **Submit**  
   `submit_onchain(request)` → `request_id`, `expires_at`.

4. **Track to fulfillment**  
   `wait_for_request_fulfillment(request_id, poll_interval, expires_at)` → `fulfillment` (includes `seal`).

5. **Consume fulfillment elsewhere**  
   Build `journal` and call `ComplianceHook.beforeTrade(user, productId, amount, journal, seal)`.  
   The Solidity contract verifies the proof and only allows trading if the zk‑verified program says `allowed == true`.

