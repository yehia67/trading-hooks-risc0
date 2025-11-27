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

/// @title Interface for an RWA compliance trading hook.
/// @notice The hook is called by a trading venue before executing a trade.
interface IComplianceHook {
    /// @notice Check whether a user is allowed to trade a given product.
    /// @param user Address of the trader.
    /// @param productId Identifier of the RWA product (e.g. GOLD_US, STOCK_XYZ).
    /// @param amount Amount the user intends to trade.
    /// @param journal ABI-encoded journal produced by the RISC Zero guest.
    /// @param seal Zero-knowledge proof (seal) returned by the verifier.
    function beforeTrade(
        address user,
        bytes32 productId,
        uint256 amount,
        bytes calldata journal,
        bytes calldata seal
    ) external;
}
