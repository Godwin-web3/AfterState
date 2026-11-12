// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {AfterStateRegistry} from "../contracts/AfterStateRegistry.sol";
import {AfterStateEIP712Verifier} from "../contracts/AfterStateEIP712Verifier.sol";
import {IAfterStateVerifier} from "../contracts/IAfterStateVerifier.sol";

interface Vm {
    function addr(uint256 privateKey) external returns (address);
    function sign(uint256 privateKey, bytes32 digest) external returns (uint8 v, bytes32 r, bytes32 s);
    function prank(address caller) external;
    function warp(uint256 timestamp) external;
    function chainId(uint256 newChainId) external;
    function etch(address target, bytes calldata code) external;
}

contract AfterStateTest {
    Vm private constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));

    uint256 private constant OWNER_KEY = 0xA11CE;
    uint256 private constant SIGNER_KEY = 0xB0B;
    address private owner;
    address private signer;
    address private recipient = address(0xCAFE);
    address private emitter = address(0xBEEF);
    address private asset = address(0xA55E7);
    bytes32 private constant AGREEMENT = keccak256("agreement-1");
    bytes32 private constant EVENT_SCHEMA = keccak256("Payment(address,address,uint256)");

    AfterStateEIP712Verifier private verifier;
    AfterStateRegistry private registry;

    function setUp() public {
        owner = vm.addr(OWNER_KEY);
        signer = vm.addr(SIGNER_KEY);
        verifier = new AfterStateEIP712Verifier(owner, 1, signer);
        registry = new AfterStateRegistry(owner, IAfterStateVerifier(address(verifier)));
        vm.prank(owner);
        registry.createAgreement(
            AGREEMENT,
            84532,
            emitter,
            EVENT_SCHEMA,
            asset,
            recipient,
            100,
            1000,
            uint64(block.timestamp + 1 days)
        );
    }

    function _fact(bytes32 nonce) internal view returns (IAfterStateVerifier.SourceFact memory fact) {
        fact = IAfterStateVerifier.SourceFact({
            sourceChainId: 84532,
            emitter: emitter,
            eventSchema: EVENT_SCHEMA,
            agreementId: AGREEMENT,
            sourceBlockHash: keccak256("source-block"),
            sourceTxHash: keccak256("source-transaction"),
            sourceBlockNumber: 123456,
            logIndex: 2,
            asset: asset,
            amount: 500,
            recipient: recipient,
            observedAt: uint64(block.timestamp),
            expiresAt: uint64(block.timestamp + 1 hours),
            keyVersion: 1,
            agreementVersion: 1,
            finalityPolicyId: keccak256("base-sepolia-finalized"),
            nonce: nonce
        });
    }

    function _signature(IAfterStateVerifier.SourceFact memory fact)
        internal
        returns (bytes memory)
    {
        bytes32 digest = verifier.digestFor(address(registry), fact);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(SIGNER_KEY, digest);
        return abi.encodePacked(r, s, v);
    }

    function testValidFactCanBeConsumedOnce() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-1"));
        registry.consumeSourceFact(fact, _signature(fact));
        require(registry.consumedNonces(keccak256(abi.encode(block.chainid, address(registry), AGREEMENT, fact.nonce))));
    }

    function testReplayIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-2"));
        bytes memory proof = _signature(fact);
        registry.consumeSourceFact(fact, proof);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, proof))
        );
        require(!ok, "second consumption unexpectedly succeeded");
    }

    function testWrongRegistryDomainIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-3"));
        bytes32 wrongDigest = verifier.digestFor(address(0x1234), fact);
        (uint8 v, bytes32 r, bytes32 s) = vm.sign(SIGNER_KEY, wrongDigest);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, abi.encodePacked(r, s, v)))
        );
        require(!ok, "signature for another registry was accepted");
    }

    function testWrongEmitterIsRejectedBeforeVerification() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-4"));
        fact.emitter = address(0xDEAD);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, _signature(fact)))
        );
        require(!ok, "wrong emitter was accepted");
    }

    function testRevokedSignerCannotVerify() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-5"));
        bytes memory proof = _signature(fact);
        vm.prank(owner);
        verifier.revokeSigner(1);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, proof))
        );
        require(!ok, "revoked signer was accepted");
    }

    function testExpiredFactIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-6"));
        bytes memory proof = _signature(fact);
        vm.warp(block.timestamp + 2 hours);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, proof))
        );
        require(!ok, "expired fact was accepted");
    }

    function testSameSourceLogCannotBeConsumedWithDifferentNonce() public {
        IAfterStateVerifier.SourceFact memory first = _fact(keccak256("source-log-nonce-a"));
        IAfterStateVerifier.SourceFact memory second = _fact(keccak256("source-log-nonce-b"));
        registry.consumeSourceFact(first, _signature(first));
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (second, _signature(second)))
        );
        require(!ok, "same source log was consumed twice with different nonces");
    }

    function testWrongAgreementVersionIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("wrong-agreement-version"));
        fact.agreementVersion = 2;
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, _signature(fact)))
        );
        require(!ok, "wrong agreement version was accepted");
    }

    function testZeroFinalityPolicyIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("zero-finality-policy"));
        fact.finalityPolicyId = bytes32(0);
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, _signature(fact)))
        );
        require(!ok, "zero finality policy was accepted");
    }

    function testAgreementIdCannotBeReusedAfterDeactivation() public {
        vm.prank(owner);
        registry.deactivateAgreement(AGREEMENT);
        vm.prank(owner);
        (bool ok,) = address(registry).call(
            abi.encodeWithSignature(
                "createAgreement(bytes32,uint256,address,bytes32,address,address,uint128,uint128,uint64)",
                AGREEMENT, 84532, emitter, EVENT_SCHEMA, asset, recipient,
                uint128(100), uint128(1000), uint64(block.timestamp + 1 days)
            )
        );
        require(!ok, "deactivated agreement ID was reused");
    }


    function testDigestMatchesRustEip712Fixture() public {
        vm.chainId(84532);
        vm.warp(1_700_000_000);

        address verifierAddress = address(0x1001);
        vm.etch(verifierAddress, type(AfterStateEIP712Verifier).runtimeCode);

        IAfterStateVerifier.SourceFact memory fact = IAfterStateVerifier.SourceFact({
            sourceChainId: 84532,
            emitter: address(0xBEEF),
            eventSchema: keccak256("Payment(address,address,uint256)"),
            agreementId: keccak256("agreement-1"),
            sourceBlockHash: keccak256("source-block"),
            sourceTxHash: keccak256("source-transaction"),
            sourceBlockNumber: 123456,
            logIndex: 2,
            asset: address(0x0A55E7),
            amount: 500,
            recipient: address(0xCAFE),
            observedAt: 1_700_000_000,
            expiresAt: 1_700_003_600,
            keyVersion: 1,
            agreementVersion: 1,
            finalityPolicyId: keccak256("base-sepolia-finalized"),
            nonce: keccak256("nonce-1")
        });

        bytes32 digest = AfterStateEIP712Verifier(verifierAddress).digestFor(address(0x2002), fact);
        require(
            digest == 0x2777312f3d33227093724b91f52fcfbe122f595e2f5dcdff63f2ed6e37e4c463,
            "Rust EIP-712 digest mismatch"
        );
    }

    function testWrongAmountIsRejected() public {
        IAfterStateVerifier.SourceFact memory fact = _fact(keccak256("nonce-7"));
        fact.amount = 2000;
        (bool ok,) = address(registry).call(
            abi.encodeCall(AfterStateRegistry.consumeSourceFact, (fact, _signature(fact)))
        );
        require(!ok, "out-of-range amount was accepted");
    }
}
