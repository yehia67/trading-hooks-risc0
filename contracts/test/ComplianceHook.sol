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

import {Test} from "forge-std/Test.sol";
import {RiscZeroCheats} from "risc0/test/RiscZeroCheats.sol";
import {Receipt as RiscZeroReceipt} from "risc0/IRiscZeroVerifier.sol";
import {RiscZeroMockVerifier} from "risc0/test/RiscZeroMockVerifier.sol";
import {VerificationFailed} from "risc0/IRiscZeroVerifier.sol";
import {ComplianceHook} from "../src/ComplianceHook.sol";
import {ImageID} from "../src/ImageID.sol";

contract ComplianceHookTest is RiscZeroCheats, Test {
    ComplianceHook public hook;
    RiscZeroMockVerifier public verifier;

    address public user;
    bytes32 public productId;
    uint256 public amount;

    function setUp() public {
        verifier = new RiscZeroMockVerifier(0);
        hook = new ComplianceHook(verifier);
        user = address(0x1234);
        productId = bytes32(uint256(1));
        amount = 100;
    }

    function _buildJournal(address journalUser, bytes32 journalProductId, bool allowed)
        internal
        pure
        returns (bytes memory)
    {
        return abi.encode(journalUser, journalProductId, allowed);
    }

    function test_AllowsWhenAllowedTrue() public {
        bytes memory journal = _buildJournal(user, productId, true);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.COMPLIANCE_ID, sha256(journal));

        hook.beforeTrade(user, productId, amount, journal, receipt.seal);
    }

    function test_RevertWhenUserNotAllowed() public {
        bytes memory journal = _buildJournal(user, productId, false);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.COMPLIANCE_ID, sha256(journal));

        vm.expectRevert("ComplianceHook: user not allowed");
        hook.beforeTrade(user, productId, amount, journal, receipt.seal);
    }

    function test_RevertWhenUserMismatch() public {
        address otherUser = address(0x5678);
        bytes memory journal = _buildJournal(otherUser, productId, true);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.COMPLIANCE_ID, sha256(journal));

        vm.expectRevert("ComplianceHook: user mismatch");
        hook.beforeTrade(user, productId, amount, journal, receipt.seal);
    }

    // Try using a proof with a mismatched journal digest.
    function test_RejectInvalidProof() public {
        bytes memory journal = _buildJournal(user, productId, true);
        bytes memory otherJournal = _buildJournal(user, bytes32(uint256(2)), true);
        RiscZeroReceipt memory receipt = verifier.mockProve(ImageID.COMPLIANCE_ID, sha256(otherJournal));

        vm.expectRevert(VerificationFailed.selector);
        hook.beforeTrade(user, productId, amount, journal, receipt.seal);
    }
}
