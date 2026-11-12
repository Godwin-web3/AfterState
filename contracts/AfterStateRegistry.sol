// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IAfterStateVerifier} from "./IAfterStateVerifier.sol";

/// @title AfterStateRegistry
/// @notice Agreement-bound source-fact verification and one-time consumption.
/// @dev Not a bridge; does not independently prove source-chain consensus.
contract AfterStateRegistry {
    error NotOwner();
    error InvalidAgreement();
    error AgreementExists();
    error AgreementInactive();
    error InvalidFact();
    error FactAlreadyConsumed();
    error Expired();
    error InvalidVerifier();

    struct Agreement {
        uint256 sourceChainId;
        uint256 destinationChainId;
        address sourceEmitter;
        bytes32 eventSchema;
        address asset;
        address recipient;
        uint128 minAmount;
        uint128 maxAmount;
        uint64 deadline;
        bool active;
    }

    address public owner;
    IAfterStateVerifier public verifier;
    mapping(bytes32 => Agreement) public agreements;
    mapping(bytes32 => bool) public consumedNonces;
    mapping(bytes32 => bool) public consumedSourceLogs;
    mapping(bytes32 => bool) public agreementEverCreated;
    mapping(bytes32 => IAfterStateVerifier.SourceFact) private acceptedFacts;

    event AgreementCreated(bytes32 indexed agreementId, uint256 sourceChainId, uint256 destinationChainId);
    event SourceFactConsumed(bytes32 indexed agreementId, bytes32 indexed nonce, bytes32 indexed sourceTxHash, uint256 logIndex);
    event VerifierUpdated(address indexed previousVerifier, address indexed newVerifier);
    event AgreementDeactivated(bytes32 indexed agreementId);

    modifier onlyOwner() { if (msg.sender != owner) revert NotOwner(); _; }

    constructor(address initialOwner, IAfterStateVerifier initialVerifier) {
        if (initialOwner == address(0) || address(initialVerifier) == address(0)) revert InvalidVerifier();
        owner = initialOwner;
        verifier = initialVerifier;
    }

    function createAgreement(
        bytes32 agreementId, uint256 sourceChainId, address sourceEmitter,
        bytes32 eventSchema, address asset, address recipient,
        uint128 minAmount, uint128 maxAmount, uint64 deadline
    ) external onlyOwner {
        if (agreementId == bytes32(0) || sourceChainId == 0 || sourceEmitter == address(0)
            || eventSchema == bytes32(0) || recipient == address(0) || minAmount == 0
            || maxAmount < minAmount || deadline <= block.timestamp) revert InvalidAgreement();
        if (agreementEverCreated[agreementId]) revert AgreementExists();
        agreementEverCreated[agreementId] = true;
        agreements[agreementId] = Agreement(sourceChainId, block.chainid, sourceEmitter,
            eventSchema, asset, recipient, minAmount, maxAmount, deadline, true);
        emit AgreementCreated(agreementId, sourceChainId, block.chainid);
    }

    /// @notice Verify and consume one source fact. Destination settlement is a
    /// separate milestone and must not be inferred from this event.
    function consumeSourceFact(IAfterStateVerifier.SourceFact calldata fact, bytes calldata proof) external {
        Agreement storage a = agreements[fact.agreementId];
        if (!a.active) revert AgreementInactive();
        if (block.timestamp > a.deadline || block.timestamp > fact.expiresAt) revert Expired();
        if (fact.sourceChainId != a.sourceChainId || fact.emitter != a.sourceEmitter
            || fact.eventSchema != a.eventSchema || fact.asset != a.asset
            || fact.recipient != a.recipient || fact.amount < a.minAmount || fact.amount > a.maxAmount
            || fact.sourceBlockHash == bytes32(0) || fact.sourceTxHash == bytes32(0)
            || fact.nonce == bytes32(0) || fact.observedAt > block.timestamp
            || fact.expiresAt <= fact.observedAt) revert InvalidFact();

        bytes32 nonceKey = keccak256(abi.encode(block.chainid, address(this), fact.agreementId, fact.nonce));
        bytes32 logKey = keccak256(abi.encode(fact.sourceChainId, fact.sourceBlockHash,
            fact.sourceTxHash, fact.logIndex, fact.emitter));
        if (consumedNonces[nonceKey] || consumedSourceLogs[logKey]) revert FactAlreadyConsumed();
        if (!verifier.verify(fact, proof)) revert InvalidFact();
        consumedNonces[nonceKey] = true;
        consumedSourceLogs[logKey] = true;
        acceptedFacts[nonceKey] = fact;
        emit SourceFactConsumed(fact.agreementId, fact.nonce, fact.sourceTxHash, fact.logIndex);
    }

    /// @notice Returns the exact source fact accepted for an agreement nonce.
    /// @dev Consumers must still apply their own destination postconditions.
    function getAcceptedFact(bytes32 agreementId, bytes32 nonce)
        external
        view
        returns (IAfterStateVerifier.SourceFact memory fact, bool exists)
    {
        bytes32 nonceKey = keccak256(abi.encode(block.chainid, address(this), agreementId, nonce));
        return (acceptedFacts[nonceKey], consumedNonces[nonceKey]);
    }

    function deactivateAgreement(bytes32 agreementId) external onlyOwner {
        Agreement storage a = agreements[agreementId];
        if (!a.active) revert AgreementInactive();
        a.active = false;
        emit AgreementDeactivated(agreementId);
    }

    function setVerifier(IAfterStateVerifier newVerifier) external onlyOwner {
        if (address(newVerifier) == address(0)) revert InvalidVerifier();
        address previous = address(verifier);
        verifier = newVerifier;
        emit VerifierUpdated(previous, address(newVerifier));
    }
}
