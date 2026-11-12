// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

interface VmAfterStateDeploy {
    function addr(uint256 privateKey) external returns (address);
    function envAddress(string calldata name) external returns (address);
    function envUint(string calldata name) external returns (uint256);
    function getChainId() external view returns (uint256);
    function getNonce(address account) external view returns (uint64);
    function startBroadcast(uint256 privateKey) external;
    function stopBroadcast() external;
}

abstract contract AfterStateDeployBase {
    VmAfterStateDeploy internal constant vm =
        VmAfterStateDeploy(address(uint160(uint256(keccak256("hevm cheat code")))));

    uint256 internal constant BASE_SEPOLIA_CHAIN_ID = 84_532;
    uint256 internal constant ETHEREUM_SEPOLIA_CHAIN_ID = 11_155_111;
    bytes32 internal constant SOURCE_PAYMENT_EVENT_SCHEMA =
        keccak256("SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)");

    function _deployerAndKey() internal returns (address deployer, uint256 privateKey) {
        privateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        deployer = vm.addr(privateKey);
    }

    function _assertTokenDeploymentNonce(address deployer) internal {
        uint256 expectedNonce = vm.envUint("TOKEN_DEPLOYER_NONCE");
        uint64 actualNonce = vm.getNonce(deployer);
        require(actualNonce == expectedNonce, "deployer nonce differs from TOKEN_DEPLOYER_NONCE");
    }
}
