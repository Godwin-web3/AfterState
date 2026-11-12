// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateSourceEmitter} from "../contracts/AfterStateSourceEmitter.sol";

interface VmSource {
    struct Log {
        bytes32[] topics;
        bytes data;
        address emitter;
    }
    function recordLogs() external;
    function getRecordedLogs() external returns (Log[] memory);
}

contract SourceMockERC20 {
    mapping(address => uint256) public balanceOf;
    mapping(address => mapping(address => uint256)) public allowance;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        allowance[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(allowance[from][msg.sender] >= amount, "allowance");
        require(balanceOf[from] >= amount, "balance");
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract AfterStateSourceEmitterTest {
    VmSource private constant vm = VmSource(address(uint160(uint256(keccak256("hevm cheat code")))));

    bytes32 private constant AGREEMENT = keccak256("agreement-a");
    bytes32 private constant NONCE = keccak256("source-payment-nonce");
    address private constant PAYER = address(0xA11CE);
    address private constant RECIPIENT = address(0xB0B);
    uint256 private constant AMOUNT = 750;

    AfterStateSourceEmitter private emitter;
    SourceMockERC20 private token;

    function setUp() public {
        emitter = new AfterStateSourceEmitter();
        token = new SourceMockERC20();
        token.mint(PAYER, 2_000);
    }

    function testPaymentTransfersExactAmountAndEmitsCanonicalLog() public {
        vm.recordLogs();
        // prank is not needed: the test contract is payer and owns the minted tokens.
        token.mint(address(this), AMOUNT);
        token.approve(address(emitter), AMOUNT);
        emitter.pay(AGREEMENT, 1, address(token), RECIPIENT, AMOUNT, NONCE);

        require(token.balanceOf(RECIPIENT) == AMOUNT, "recipient amount");
        VmSource.Log[] memory logs = vm.getRecordedLogs();
        require(logs.length == 1, "one source event");

        VmSource.Log memory entry = logs[0];
        require(entry.emitter == address(emitter), "wrong emitter");
        require(
            entry.topics[0] ==
                keccak256(
                    "SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)"
                ),
            "wrong topic0"
        );
        require(entry.topics[1] == AGREEMENT, "wrong agreement topic");
        require(entry.topics[2] == bytes32(uint256(1)), "wrong version topic");
        require(entry.topics[3] == bytes32(uint256(uint160(address(token)))), "wrong asset topic");

        (address payer, address recipient, uint256 amount, uint64 sourceTimestamp, bytes32 nonce) =
            abi.decode(entry.data, (address, address, uint256, uint64, bytes32));
        require(payer == address(this), "wrong payer");
        require(recipient == RECIPIENT, "wrong recipient");
        require(amount == AMOUNT, "wrong amount");
        require(sourceTimestamp == uint64(block.timestamp), "wrong source timestamp");
        require(nonce == NONCE, "wrong nonce");
    }

    function testFeeOnTransferTokenIsRejected() public {
        FeeOnTransferToken feeToken = new FeeOnTransferToken();
        feeToken.mint(address(this), AMOUNT);
        feeToken.approve(address(emitter), AMOUNT);
        (bool ok,) = address(emitter).call(
            abi.encodeCall(
                AfterStateSourceEmitter.pay,
                (AGREEMENT, 1, address(feeToken), RECIPIENT, AMOUNT, NONCE)
            )
        );
        require(!ok, "fee-on-transfer token unexpectedly accepted");
        require(feeToken.balanceOf(RECIPIENT) == 0, "transfer should revert atomically");
    }

    function testZeroNonceIsRejected() public {
        token.mint(address(this), AMOUNT);
        token.approve(address(emitter), AMOUNT);
        (bool ok,) = address(emitter).call(
            abi.encodeCall(
                AfterStateSourceEmitter.pay,
                (AGREEMENT, 1, address(token), RECIPIENT, AMOUNT, bytes32(0))
            )
        );
        require(!ok, "zero nonce accepted");
    }
}

contract FeeOnTransferToken is SourceMockERC20 {
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        require(allowance[from][msg.sender] >= amount, "allowance");
        require(balanceOf[from] >= amount, "balance");
        allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount - 1;
        return true;
    }
}
