// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IAfterStateVerifier} from "./IAfterStateVerifier.sol";

/// @title AfterStateEIP712Verifier
/// @notice Verifies EIP-712 attestations from versioned, explicitly authorized
///         signer keys. This authenticates the attester, not the truth of the
///         source-chain observation.
contract AfterStateEIP712Verifier is IAfterStateVerifier {
    error NotOwner();
    error InvalidSigner();
    error InvalidVersion();
    error VersionAlreadyConfigured();
    error SignerAlreadyRevoked();
    error InvalidSignature();

    address public owner;
    mapping(uint32 => address) public signerAtVersion;
    mapping(uint32 => bool) public revokedVersion;

    bytes32 public constant SOURCE_FACT_TYPEHASH = keccak256(
        "SourceFact(uint256 sourceChainId,address emitter,bytes32 eventSchema,bytes32 agreementId,bytes32 sourceBlockHash,bytes32 sourceTxHash,uint256 sourceBlockNumber,uint256 logIndex,address asset,uint256 amount,address recipient,uint64 observedAt,uint64 expiresAt,uint32 keyVersion,uint32 agreementVersion,bytes32 finalityPolicyId,bytes32 nonce,address destinationRegistry)"
    );
    bytes32 public constant DOMAIN_TYPEHASH = keccak256(
        "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)"
    );
    bytes32 public constant NAME_HASH = keccak256("AfterState");
    bytes32 public constant VERSION_HASH = keccak256("1");

    event SignerConfigured(uint32 indexed version, address indexed signer);
    event SignerRevoked(uint32 indexed version, address indexed signer);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotOwner();
        _;
    }

    constructor(address initialOwner, uint32 initialVersion, address initialSigner) {
        if (initialOwner == address(0) || initialSigner == address(0) || initialVersion == 0) {
            revert InvalidSigner();
        }
        owner = initialOwner;
        signerAtVersion[initialVersion] = initialSigner;
        emit SignerConfigured(initialVersion, initialSigner);
    }

    function configureSigner(uint32 version, address signer) external onlyOwner {
        if (version == 0 || signer == address(0)) revert InvalidSigner();
        if (signerAtVersion[version] != address(0)) revert VersionAlreadyConfigured();
        signerAtVersion[version] = signer;
        emit SignerConfigured(version, signer);
    }

    function revokeSigner(uint32 version) external onlyOwner {
        address signer = signerAtVersion[version];
        if (signer == address(0)) revert InvalidVersion();
        if (revokedVersion[version]) revert SignerAlreadyRevoked();
        revokedVersion[version] = true;
        emit SignerRevoked(version, signer);
    }

    function transferOwnership(address newOwner) external onlyOwner {
        if (newOwner == address(0)) revert InvalidSigner();
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    /// @notice Computes the digest expected for a particular destination
    ///         registry. Exposing this helps off-chain signers construct the
    ///         exact typed-data payload.
    function digestFor(address destinationRegistry, SourceFact calldata fact)
        external
        view
        returns (bytes32)
    {
        return _digest(destinationRegistry, fact);
    }

    function verify(SourceFact calldata fact, bytes calldata proof)
        external
        view
        override
        returns (bool)
    {
        address signer = signerAtVersion[fact.keyVersion];
        if (signer == address(0) || revokedVersion[fact.keyVersion] || proof.length != 65) {
            return false;
        }

        bytes32 digest = _digest(msg.sender, fact);
        bytes32 r;
        bytes32 s;
        uint8 v;
        assembly {
            r := calldataload(proof.offset)
            s := calldataload(add(proof.offset, 32))
            v := byte(0, calldataload(add(proof.offset, 64)))
        }

        // Enforce canonical low-s signatures and valid recovery identifiers.
        if (uint256(s) > 0x7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0) {
            return false;
        }
        if (v != 27 && v != 28) return false;
        return ecrecover(digest, v, r, s) == signer;
    }

    function _digest(address destinationRegistry, SourceFact calldata fact)
        internal
        view
        returns (bytes32)
    {
        bytes32 structHash = keccak256(
            abi.encode(
                SOURCE_FACT_TYPEHASH,
                fact.sourceChainId,
                fact.emitter,
                fact.eventSchema,
                fact.agreementId,
                fact.sourceBlockHash,
                fact.sourceTxHash,
                fact.sourceBlockNumber,
                fact.logIndex,
                fact.asset,
                fact.amount,
                fact.recipient,
                fact.observedAt,
                fact.expiresAt,
                fact.keyVersion,
                fact.agreementVersion,
                fact.finalityPolicyId,
                fact.nonce,
                destinationRegistry
            )
        );
        bytes32 domainSeparator = keccak256(
            abi.encode(DOMAIN_TYPEHASH, NAME_HASH, VERSION_HASH, block.chainid, address(this))
        );
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, structHash));
    }
}
