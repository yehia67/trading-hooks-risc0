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

pragma solidity ^0.8.20;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol"; // auto-generated after running `cargo build`.
import {IComplianceHook} from "./IComplianceHook.sol";

/// @title RWA Compliance Trading Hook
/// @notice Uses a RISC Zero proof to decide whether a user is allowed to trade a product.
/// @dev The guest program computes a boolean `allowed` based on off-chain compliance data
///      and commits `(user, productId, allowed)` to the journal. The hook verifies the
///      proof and checks that `allowed == true` for the given user and product.
contract ComplianceHook is IComplianceHook {
    /// @notice RISC Zero verifier contract address.
    IRiscZeroVerifier public immutable VERIFIER;

    /// @notice Image ID of the compliance guest program.
    bytes32 public constant IMAGE_ID = ImageID.COMPLIANCE_ID;

    constructor(IRiscZeroVerifier _verifier) {
        VERIFIER = _verifier;
    }

    function beforeTrade(
        address user,
        bytes32 productId,
        uint256 amount,
        bytes calldata journal,
        bytes calldata seal
    ) external override {
        VERIFIER.verify(seal, IMAGE_ID, sha256(journal));

        (address journalUser, bytes32 journalProductId, bool allowed) =
            abi.decode(journal, (address, bytes32, bool));

        require(journalUser == user, "ComplianceHook: user mismatch");
        require(journalProductId == productId, "ComplianceHook: product mismatch");
        require(allowed, "ComplianceHook: user not allowed");


        amount; 
    }
}
