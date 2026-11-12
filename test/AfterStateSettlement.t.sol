// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateRegistry} from "../contracts/AfterStateRegistry.sol";
import {AfterStateEIP712Verifier} from "../contracts/AfterStateEIP712Verifier.sol";
import {AfterStateERC20Settlement, IAfterStateFactRegistry} from "../contracts/AfterStateERC20Settlement.sol";
import {IAfterStateVerifier} from "../contracts/IAfterStateVerifier.sol";

interface VmSettlement {
    function addr(uint256 privateKey) external returns (address);
    function sign(uint256 privateKey, bytes32 digest) external returns (uint8 v, bytes32 r, bytes32 s);
    function prank(address caller) external;
}

contract MockExactERC20 {
    mapping(address => uint256) public balanceOf;

    function mint(address to, uint256 amount) external {
        balanceOf[to] += amount;
    }

    function transfer(address to, uint256 amount) external returns (bool) {
        require(balanceOf[msg.sender] >= amount, "balance");
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        return true;
    }
}

contract AfterStateSettlementTest {
    VmSettlement private constant vm =
        VmSettlement(address(uint160(uint256(keccak256("hevm cheat code")))));

    uint256 private constant OWNER_KEY = 0xA11CE;
    uint256 private constant SIGNER_KEY = 0xB0B;
    address private owner;
    address private recipient = address(0xCAFE);
    address private emitter = address(0xBEEF);
    bytes32 private constant AGREEMENT = keccak256("settlement-agreement");
    bytes32 private constant EVENT_SCHEMA = keccak256("Payment(address,address,uint256)");

    MockExactERC20 private token;
    AfterStateEIP712Verifier private verifier;
    AfterStateRegistry private registry;
    AfterStateERC20Settlement private settlement;

    function setUp() public {
        owner = vm.addr(OWNER_KEY);
        token = new MockExactERC20();
        verifier = new AfterStateEIP712Verifier(owner, 1, vm.addr(SIGNER_KEY));
        registry = new AfterStateRegistry(owner, IAfterStateVerifier(address(verifier)));
        settlement = new AfterStateERC20Settlement(
            IAfterStateFactRegistry(address(registry))
        );
        vm.prank(owner);
        registry.setSettlementAdapter(address(settlement));

        vm.prank(owner);
        registry.createAgreement(
            AGREEMENT,
            84532,
            emitter,
            EVENT_SCHEMA,
            address(token),
            recipient,
            100,
            1000,
            uint64(block.timestamp + 1 days)
        );
    }

    function _attest(bytes32 nonce) internal {
        IAfterStateVerifier.SourceFact memory fact = IAfterStateVerifier.SourceFact({
            sourceChainId: 84532,
            emitter: emitter,
            eventSchema: EVENT_SCHEMA,
            agreementId: AGREEMENT,
            sourceBlockHash: keccak256("settlement-source-block"),
            sourceTxHash: keccak256(abi.encodePacked("settlement-source-tx", nonce)),
            sourceBlockNumber: 123456,
            logIndex: 0,
            asset: address(token),
            amount: 500,
            recipient: recipient,
            observedAt: uint64(block.timestamp),
            expiresAt: uint64(block.timestamp + 1 hours),
            keyVersion: 1,
            agreementVersion: 1,
            finalityPolicyId: keccak256("base-sepolia-finalized"),
            nonce: nonce
        });
        bytes32 digest = verifier.digestFor(address(registry), fact);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(SIGNER_KEY, digest);
        registry.consumeSourceFact(fact, abi.encodePacked(r, s, v));
    }

    function testSettlementChecksAndDeliversExactOutcome() public {
        bytes32 nonce = keccak256("settlement-nonce-1");
        _attest(nonce);
        token.mint(address(settlement), 500);

        settlement.settleERC20(AGREEMENT, nonce);

        require(token.balanceOf(recipient) == 500, "recipient did not receive exact amount");
        require(token.balanceOf(address(settlement)) == 0, "escrow balance mismatch");
        require(settlement.settled(keccak256(abi.encode(AGREEMENT, nonce))), "not marked settled");
    }

    function testCannotSettleWithoutAcceptedSourceFact() public {
        (bool ok,) = address(settlement).call(
            abi.encodeCall(AfterStateERC20Settlement.settleERC20, (AGREEMENT, keccak256("unknown")))
        );
        require(!ok, "unattested settlement succeeded");
    }

    function testSecondSettlementAdapterCannotClaimSameFact() public {
        bytes32 nonce = keccak256("settlement-nonce-rogue");
        _attest(nonce);
        AfterStateERC20Settlement rogue = new AfterStateERC20Settlement(
            IAfterStateFactRegistry(address(registry))
        );
        token.mint(address(rogue), 500);
        (bool ok,) = address(rogue).call(
            abi.encodeCall(AfterStateERC20Settlement.settleERC20, (AGREEMENT, nonce))
        );
        require(!ok, "unconfigured adapter claimed a fact");
        require(token.balanceOf(address(rogue)) == 500, "rogue adapter transferred funds");
    }

    function testCannotSettleSameFactTwice() public {
        bytes32 nonce = keccak256("settlement-nonce-2");
        _attest(nonce);
        token.mint(address(settlement), 1000);
        settlement.settleERC20(AGREEMENT, nonce);

        (bool ok,) = address(settlement).call(
            abi.encodeCall(AfterStateERC20Settlement.settleERC20, (AGREEMENT, nonce))
        );
        require(!ok, "duplicate settlement succeeded");
    }
}
