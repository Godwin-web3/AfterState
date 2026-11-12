// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateDemoToken} from "../contracts/AfterStateDemoToken.sol";
import {AfterStateEIP712Verifier} from "../contracts/AfterStateEIP712Verifier.sol";
import {AfterStateRegistry} from "../contracts/AfterStateRegistry.sol";
import {AfterStateERC20Settlement} from "../contracts/AfterStateERC20Settlement.sol";
import {IAfterStateVerifier} from "../contracts/IAfterStateVerifier.sol";
import {AfterStateDeployBase} from "./AfterStateDeployBase.sol";

/// @notice Deploy the destination-side token, verifier, registry and settlement
///         adapter to Ethereum Sepolia.
/// @dev The token must be the first CREATE from the same deployer at the same
///      nonce used by DeployBaseSepolia, producing an identical token address.
contract DeployEthereumSepolia is AfterStateDeployBase {
    function run()
        external
        returns (
            address tokenAddress,
            address verifierAddress,
            address registryAddress,
            address settlementAddress
        )
    {
        require(vm.getChainId() == ETHEREUM_SEPOLIA_CHAIN_ID, "run only on Ethereum Sepolia");
        (address deployer, uint256 privateKey) = _deployerAndKey();
        _assertTokenDeploymentNonce(deployer);

        address expectedAsset = vm.envAddress("EXPECTED_ASSET_ADDRESS");
        address signer = vm.envAddress("AFTERSTATE_MPC_SIGNER_ADDRESS");
        address sourceEmitter = vm.envAddress("BASE_SEPOLIA_SOURCE_EMITTER");
        address recipient = vm.envAddress("DESTINATION_RECIPIENT");
        bytes32 agreementId = bytes32(vm.envUint("AGREEMENT_ID"));
        uint256 minAmountRaw = vm.envUint("MIN_AMOUNT");
        uint256 maxAmountRaw = vm.envUint("MAX_AMOUNT");
        uint256 deadlineRaw = vm.envUint("AGREEMENT_DEADLINE");
        uint256 prefundAmount = vm.envUint("DESTINATION_PREFUND_AMOUNT");
        require(minAmountRaw > 0 && maxAmountRaw >= minAmountRaw, "invalid agreement amounts");
        require(maxAmountRaw <= type(uint128).max, "maximum amount exceeds uint128");
        require(deadlineRaw <= type(uint64).max, "deadline exceeds uint64");
        uint128 minAmount = uint128(minAmountRaw);
        uint128 maxAmount = uint128(maxAmountRaw);
        uint64 deadline = uint64(deadlineRaw);

        require(expectedAsset != address(0), "expected asset is zero");
        require(signer != address(0), "MPC signer address is zero");
        require(sourceEmitter != address(0), "source emitter is zero");
        require(recipient != address(0), "destination recipient is zero");
        require(agreementId != bytes32(0), "agreement ID is zero");
        require(prefundAmount >= maxAmount, "destination escrow underfunded");

        vm.startBroadcast(privateKey);
        AfterStateDemoToken token = new AfterStateDemoToken(deployer);
        require(address(token) == expectedAsset, "source/destination token addresses differ");

        AfterStateEIP712Verifier verifier = new AfterStateEIP712Verifier(deployer, 1, signer);
        AfterStateRegistry registry =
            new AfterStateRegistry(deployer, IAfterStateVerifier(address(verifier)));
        AfterStateERC20Settlement settlement = new AfterStateERC20Settlement(registry);

        registry.setSettlementAdapter(address(settlement));
        registry.createAgreement(
            agreementId,
            BASE_SEPOLIA_CHAIN_ID,
            sourceEmitter,
            SOURCE_PAYMENT_EVENT_SCHEMA,
            address(token),
            recipient,
            minAmount,
            maxAmount,
            deadline
        );
        token.mint(address(settlement), prefundAmount);
        vm.stopBroadcast();

        return (address(token), address(verifier), address(registry), address(settlement));
    }
}
