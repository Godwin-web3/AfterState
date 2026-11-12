// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @notice Verifier boundary. Signature-based implementations trust an
/// authorized attester; stronger implementations may verify source proofs.
interface IAfterStateVerifier {
    struct SourceFact {
        uint256 sourceChainId;
        address emitter;
        bytes32 eventSchema;
        bytes32 agreementId;
        bytes32 sourceBlockHash;
        bytes32 sourceTxHash;
        uint256 sourceBlockNumber;
        uint256 logIndex;
        address asset;
        uint256 amount;
        address recipient;
        uint64 observedAt;
        uint64 expiresAt;
        uint32 keyVersion;
        uint32 agreementVersion;
        bytes32 finalityPolicyId;
        bytes32 nonce;
    }

    function verify(SourceFact calldata fact, bytes calldata proof) external view returns (bool);
}
