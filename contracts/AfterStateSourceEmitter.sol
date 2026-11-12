// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

/// @title AfterStateSourceEmitter
/// @notice Emits an MPC-verifiable event only after an exact ERC-20 payment.
/// @dev This contract is an event source, not an escrow. The source payment is
///      final and is not automatically refundable if destination execution fails.
contract AfterStateSourceEmitter {
    error InvalidInput();
    error TransferFailed();
    error InexactTransfer();
    error ReentrantCall();

    uint256 private entered = 1;

    event SourcePayment(
        bytes32 indexed agreementId,
        uint32 indexed agreementVersion,
        address indexed asset,
        address payer,
        address recipient,
        uint256 amount,
        uint64 sourceTimestamp,
        bytes32 nonce
    );

    modifier nonReentrant() {
        if (entered != 1) revert ReentrantCall();
        entered = 2;
        _;
        entered = 1;
    }

    /// @notice Pay a recipient on the source chain and emit the exact payment fact.
    /// @dev The adapter must still validate this event against its configured agreement.
    function pay(
        bytes32 agreementId,
        uint32 agreementVersion,
        address asset,
        address recipient,
        uint256 amount,
        bytes32 nonce
    ) external nonReentrant {
        if (
            agreementId == bytes32(0) ||
            agreementVersion == 0 ||
            asset == address(0) ||
            recipient == address(0) ||
            amount == 0 ||
            nonce == bytes32(0) ||
            block.timestamp > type(uint64).max
        ) revert InvalidInput();

        uint256 beforeBalance = _balanceOf(asset, recipient);
        _safeTransferFrom(asset, msg.sender, recipient, amount);
        uint256 afterBalance = _balanceOf(asset, recipient);
        if (afterBalance < beforeBalance || afterBalance - beforeBalance != amount) {
            revert InexactTransfer();
        }

        emit SourcePayment(
            agreementId,
            agreementVersion,
            asset,
            msg.sender,
            recipient,
            amount,
            uint64(block.timestamp),
            nonce
        );
    }

    function _balanceOf(address token, address account) private view returns (uint256 value) {
        (bool ok, bytes memory data) = token.staticcall(
            abi.encodeWithSelector(bytes4(keccak256("balanceOf(address)")), account)
        );
        if (!ok || data.length != 32) revert TransferFailed();
        value = abi.decode(data, (uint256));
    }

    function _safeTransferFrom(address token, address from, address to, uint256 amount) private {
        (bool ok, bytes memory data) = token.call(
            abi.encodeWithSelector(
                bytes4(keccak256("transferFrom(address,address,uint256)")),
                from,
                to,
                amount
            )
        );
        if (!ok || (data.length != 0 && (data.length != 32 || !abi.decode(data, (bool))))) {
            revert TransferFailed();
        }
    }
}
