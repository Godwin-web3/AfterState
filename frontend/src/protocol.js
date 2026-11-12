import { Contract, id, randomBytes, hexlify, parseUnits, formatUnits, isAddress, ZeroAddress } from 'ethers';
const ENV = import.meta.env || {};
export const CHAINS = {
  base: { id: 84532, name: 'Base Sepolia', short: 'Base', currency: 'ETH', rpc: 'https://sepolia.base.org', explorer: 'https://sepolia.basescan.org' },
  destination: { id: 11155111, name: 'Ethereum Sepolia', short: 'Ethereum', currency: 'ETH', rpc: 'https://rpc.sepolia.org', explorer: 'https://sepolia.etherscan.io' }
};
export const SOURCE_SCHEMA = id('SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)');
export const SOURCE_EMITTER_ABI = [
 'function pay(bytes32 agreementId,uint32 agreementVersion,address asset,address recipient,uint256 amount,bytes32 nonce)',
 'event SourcePayment(bytes32 indexed agreementId,uint32 indexed agreementVersion,address indexed asset,address payer,address recipient,uint256 amount,uint64 sourceTimestamp,bytes32 nonce)'
];
export const REGISTRY_ABI = [
 'function owner() view returns (address)', 'function verifier() view returns (address)', 'function settlementAdapter() view returns (address)',
 'function agreements(bytes32) view returns (uint256 sourceChainId,uint256 destinationChainId,address sourceEmitter,bytes32 eventSchema,address asset,address recipient,uint128 minAmount,uint128 maxAmount,uint64 deadline,uint32 version,bool active)',
 'function agreementEverCreated(bytes32) view returns (bool)',
 'function createAgreement(bytes32 agreementId,uint256 sourceChainId,address sourceEmitter,bytes32 eventSchema,address asset,address recipient,uint128 minAmount,uint128 maxAmount,uint64 deadline)',
 'function getAcceptedFact(bytes32 agreementId,bytes32 nonce) view returns ((uint256 sourceChainId,address emitter,bytes32 eventSchema,bytes32 agreementId,bytes32 sourceBlockHash,bytes32 sourceTxHash,uint256 sourceBlockNumber,uint256 logIndex,address asset,uint256 amount,address recipient,uint64 observedAt,uint64 expiresAt,uint32 keyVersion,uint32 agreeme...[truncated]