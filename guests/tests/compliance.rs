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

use alloy_primitives::{Address, B256};
use alloy_sol_types::SolValue;
use guests::COMPLIANCE_ELF;
use risc0_zkvm::{default_executor, ExecutorEnv};

type Input = (Address, B256, bool, bool);
type Output = (Address, B256, bool);

#[test]
fn allows_when_kyc_and_aml_pass() {
    let user = Address::from([1u8; 20]);
    let product_id = B256::from([2u8; 32]);
    let kyc_passed = true;
    let aml_passed = true;

    let input = (user, product_id, kyc_passed, aml_passed);

    let env = ExecutorEnv::builder()
        .write_slice(&Input::abi_encode(&input))
        .build()
        .unwrap();

    // NOTE: Use the executor to run tests without proving.
    let session_info = default_executor().execute(env, COMPLIANCE_ELF).unwrap();

    let (journal_user, journal_product_id, allowed) =
        <Output>::abi_decode(&session_info.journal.bytes).unwrap();

    assert_eq!(journal_user, user);
    assert_eq!(journal_product_id, product_id);
    assert!(allowed);
}

#[test]
fn rejects_when_kyc_or_aml_fail() {
    let user = Address::from([3u8; 20]);
    let product_id = B256::from([4u8; 32]);
    let kyc_passed = true;
    let aml_passed = false;

    let input = (user, product_id, kyc_passed, aml_passed);

    let env = ExecutorEnv::builder()
        .write_slice(&Input::abi_encode(&input))
        .build()
        .unwrap();

    // NOTE: Use the executor to run tests without proving.
    let session_info = default_executor().execute(env, COMPLIANCE_ELF).unwrap();

    let (journal_user, journal_product_id, allowed) =
        <Output>::abi_decode(&session_info.journal.bytes).unwrap();

    assert_eq!(journal_user, user);
    assert_eq!(journal_product_id, product_id);
    assert!(!allowed);
}
