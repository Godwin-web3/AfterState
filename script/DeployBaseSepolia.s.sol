// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateDemoToken} from "../contracts/AfterStateDemoToken.sol";
import {AfterStateSourceEmitter} from "../contracts/AfterStateSourceEmitter.sol";
import {AfterStateDeployBase} from "./AfterStateDeployBase.sol";

/// @notice Deploy the source-side token and emitter to Base Sepolia.
/// @dev Deploy the token as the first CREATE from the dedicated deployer on
///      both Base Sepolia and Ethereum Sepolia using the same account nonce.
///      This produces the same token address on both chains.
contract DeployBaseSepolia is AfterStateDeployBase {
    function run() external returns (address tokenAddress, address emitterAddress, bytes32 tokenCodeHash) {
        require(vm.getChainId() == BASE_SEPOLIA_CHAIN_ID, "run only on Base Sepolia");
        (address deployer, uint256 privateKey) = _deployerAndKey();
        _assertTokenDeploymentNonce(deployer);

        address sourcePayer = vm.envAddress("SOURCE_PAYER");
        uint256 sourceMintAmount = vm.envUint("SOURCE_MINT_AMOUNT");
        require(sourcePayer != address(0) && sourceMintAmount > 0, "invalid source mint");

        vm.startBroadcast(privateKey);
        AfterStateDemoToken token = new AfterStateDemoToken(deployer);
        AfterStateSourceEmitter emitter = new AfterStateSourceEmitter();
        token.mint(sourcePayer, sourceMintAmount);
        vm.stopBroadcast();

        return (address(token), address(emitter), keccak256(address(token).code));
    }
}
