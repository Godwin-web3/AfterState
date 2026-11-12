// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IAfterStateVerifier} from "./IAfterStateVerifier.sol";

interface IERC20AfterState {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address to, uint256 amount) external returns (bool);
}

interface IAfterStateFactRegistry {
    function getAcceptedFact(bytes32 agreementId, bytes32 nonce)
        external
        view
        returns (IAfterStateVerifier.SourceFact memory fact, bool exists);
}

/// @title AfterStateERC20Settlement
/// @notice Releases pre-funded destination ERC-20 escrow only after a source
///         fact has been accepted by the registry. Enforces exact recipient
///         balance delta, not merely a successful token call.
/// @dev This is a narrow reference adapter, not a generic state-diff engine.
contract AfterStateERC20Settlement {
    error ReentrantCall();
    error FactNotAccepted();
    error FactAlreadySettled();
    error InvalidSettlementAsset();
    error InvalidRecipient();
    error InvalidAmount();
    error FactExpired();
    error TransferFailed();
    error PostconditionFailed();

    IAfterStateFactRegistry public immutable registry;
    mapping(bytes32 => bool) public settled;

    bool private entered;

    event DestinationOutcomeEnforced(
        bytes32 indexed agreementId,
        bytes32 indexed nonce,
        address indexed recipient,
        address asset,
        uint256 amount
    );

    constructor(IAfterStateFactRegistry sourceFactRegistry) {
        if (address(sourceFactRegistry) == address(0)) revert FactNotAccepted();
        registry = sourceFactRegistry;
    }

    modifier nonReentrant() {
        if (entered) revert ReentrantCall();
        entered = true;
        _;
        entered = false;
    }

    /// @notice Pays the exact attested asset/amount to the attested recipient.
    /// @dev Contract must be pre-funded. Fee-on-transfer and rebasing behavior
    ///      that changes the exact observed balance delta is rejected.
    function settleERC20(bytes32 agreementId, bytes32 nonce) external nonReentrant {
        bytes32 settlementKey = keccak256(abi.encode(agreementId, nonce));
        if (settled[settlementKey]) revert FactAlreadySettled();

        (IAfterStateVerifier.SourceFact memory fact, bool exists) =
            registry.getAcceptedFact(agreementId, nonce);
        if (!exists) revert FactNotAccepted();
        if (block.timestamp > fact.expiresAt) revert FactExpired();
        if (fact.asset == address(0)) revert InvalidSettlementAsset();
        if (fact.recipient == address(0)) revert InvalidRecipient();
        if (fact.amount == 0) revert InvalidAmount();

        IERC20AfterState token = IERC20AfterState(fact.asset);
        uint256 contractBefore = token.balanceOf(address(this));
        uint256 recipientBefore = token.balanceOf(fact.recipient);
        if (contractBefore < fact.amount) revert InvalidAmount();

        // Effects before interaction; a revert in the transfer/postcondition
        // rolls this write back atomically.
        settled[settlementKey] = true;
        if (!token.transfer(fact.recipient, fact.amount)) revert TransferFailed();

        uint256 contractAfter = token.balanceOf(address(this));
        uint256 recipientAfter = token.balanceOf(fact.recipient);
        if (
            contractBefore - contractAfter != fact.amount
                || recipientAfter < recipientBefore
                || recipientAfter - recipientBefore != fact.amount
        ) revert PostconditionFailed();

        emit DestinationOutcomeEnforced(
            agreementId, nonce, fact.recipient, fact.asset, fact.amount
        );
    }
}
