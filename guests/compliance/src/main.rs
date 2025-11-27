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

use std::io::Read;

use alloy_primitives::{Address, B256};
use alloy_sol_types::SolValue;
use risc0_zkvm::guest::env;


fn main() {
    let mut input_bytes = Vec::<u8>::new();
    env::stdin().read_to_end(&mut input_bytes).unwrap();

    type Input = (Address, B256, bool, bool);

    let (user, product_id, kyc_passed, aml_passed) =
        <Input>::abi_decode(&input_bytes).expect("invalid compliance input");

    let allowed = kyc_passed && aml_passed;

    type Output = (Address, B256, bool);
    let journal = <Output>::abi_encode(&(user, product_id, allowed));

    env::commit_slice(&journal);
}
