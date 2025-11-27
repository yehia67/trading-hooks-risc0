// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Duration;

use crate::even_number::IComplianceHook::IComplianceHookInstance;
use alloy::{
    primitives::{Address, Bytes, B256, U256},
    signers::local::PrivateKeySigner,
    sol_types::SolValue,
};
use anyhow::{bail, Context, Result};
use boundless_market::{Client, Deployment, StorageProviderConfig};
use clap::Parser;
use guests::COMPLIANCE_ELF;
use url::Url;

/// Timeout for the transaction to be confirmed.
pub const TX_TIMEOUT: Duration = Duration::from_secs(30);

mod even_number {
    alloy::sol!(
        #![sol(rpc, all_derives)]
        "../contracts/src/IComplianceHook.sol"
    );
}

/// Arguments of the publisher CLI.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// The number to publish to the EvenNumber contract.
    #[clap(short, long)]
    number: u32,
    /// URL of the Ethereum RPC endpoint.
    #[clap(short, long, env)]
    rpc_url: Url,
    /// Private key used to interact with the EvenNumber contract and the Boundless Market.
    #[clap(long, env)]
    private_key: PrivateKeySigner,
    /// Address of the EvenNumber contract.
    #[clap(short, long, env)]
    even_number_address: Address,
    #[clap(long, env)]
    user: Address,
    #[clap(long, env)]
    product_id: B256,
    #[clap(long, env)]
    kyc_passed: bool,
    #[clap(long, env)]
    aml_passed: bool,
    #[clap(long, env)]
    program_url: Option<Url>,
    #[clap(short, long, requires = "order_stream_url")]
    offchain: bool,
    #[clap(flatten, next_help_heading = "Storage Provider")]
    storage_config: StorageProviderConfig,

    #[clap(flatten, next_help_heading = "Boundless Market Deployment")]
    deployment: Option<Deployment>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    match dotenvy::dotenv() {
        Ok(path) => tracing::debug!("Loaded environment variables from {:?}", path),
        Err(e) if e.not_found() => tracing::debug!("No .env file found"),
        Err(e) => bail!("failed to load .env file: {}", e),
    }
    let args = Args::parse();

    let client = Client::builder()
        .with_rpc_url(args.rpc_url)
        .with_deployment(args.deployment)
        .with_storage_provider_config(&args.storage_config)?
        .with_private_key(args.private_key)
        .build()
        .await
        .context("failed to build boundless client")?;

    tracing::info!("Number to publish: {}", args.number);
    type Input = (Address, B256, bool, bool);
    let input = (args.user, args.product_id, args.kyc_passed, args.aml_passed);
    let input_bytes = <Input>::abi_encode(&input);

    let request = if let Some(program_url) = args.program_url {
        // Use the provided URL
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

    let (request_id, expires_at) = client.submit_onchain(request).await?;

    tracing::info!("Waiting for request {:x} to be fulfilled", request_id);
    let fulfillment = client
        .wait_for_request_fulfillment(request_id, Duration::from_secs(5), expires_at)
        .await?;
    tracing::info!("Request {:x} fulfilled", request_id);

    let allowed = args.kyc_passed && args.aml_passed;
    type Output = (Address, B256, bool);
    let journal_bytes = <Output>::abi_encode(&(args.user, args.product_id, allowed));
    let journal = Bytes::from(journal_bytes);

    let hook = IComplianceHookInstance::new(args.even_number_address, client.provider().clone());
    let call_before_trade = hook
        .beforeTrade(
            args.user,
            args.product_id,
            U256::from(args.number),
            journal,
            fulfillment.seal,
        )
        .from(client.caller());

    tracing::info!("Calling ComplianceHook beforeTrade function");
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

    Ok(())
}
