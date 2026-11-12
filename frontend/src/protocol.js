import { id, isAddress, ZeroAddress } from 'ethers';
const env = import.meta.env || {};
export const CHAINS = {
  base: {id:84532,name:'Base Sepolia',rpc:'https://sepolia.base.org',explorer:'https://sepolia.basescan.org'},
  destination: {id:11155111,name:'Ethereum Sepolia',rpc:'https://rpc.sepolia.org',explorer:'https://sepolia.etherscan.io'}
};
export const SOURCE_SCHEMA = id('SourcePayment(bytes32,uint32,address,address,address,uint256,uint64,bytes32)');
export const SOURCE_ABI = [
  'function pay(bytes32 agreementId,uint32 agreementVersion,address asset,address recipient,uint256 amount,bytes32 nonce)',
  'event SourcePayment(bytes32 indexed agreementId,uint32 indexed agreementVersion,address indexed asset,address payer,address recipient,uint256 amount,uint64 sourceTimestamp,bytes32 nonce)'
];
export const REGISTRY_ABI = [
  'function owner() view returns (address)',
  'function verifier() view returns (address)',
  'function settlementAdapter() view returns (address)',
  'function createAgreement(bytes32,uint256,address,bytes32,address,address,uint128,uint128,uint64)',
  'function consumeSourceFact((uint256 sourceChainId,address emitter,bytes32 eventSchema,bytes32 agreementId,bytes32 sourceBlockHash,bytes32 sourceTxHash,uint256 sourceBlockNumber,uint256 logIndex,address asset,uint256 amount,address recipient,uint64 observedAt,uint64 expiresAt,uint32 keyVersion,uint32 agreementVersion,bytes32 finalityPolicyId,bytes32 nonce) fact,bytes proof)',
  'function getAcceptedFact(bytes32,bytes32) view returns ((uint256 sourceChainId,address emitter,bytes32 eventSchema,bytes32 agreementId,bytes32 sourceBlockHash,bytes32 sourceTxHash,uint256 sourceBlockNumber,uint256 logIndex,address asset,uint256 amount,address recipient,uint64 observedAt,uint64 expiresAt,uint32 keyVersion,uint32 agreementVersion,bytes32 finalityPolicyId,bytes32 nonce) fact,bool exists)'
];
export const TOKEN_ABI = [
  'function symbol() view returns (string)','function decimals() view returns (uint8)',
  'function allowance(address,address) view returns (uint256)','function approve(address,uint256) returns (bool)',
  'function mint(address,uint256)','function owner() view returns (address)',
  'function transfer(address,uint256) returns (bool)'
];
export const SETTLEMENT_ABI = ['function settleERC20(bytes32,bytes32)'];
export const DEFAULT_CONFIG = {
  sourceToken:env.VITE_SOURCE_TOKEN||'',sourceEmitter:env.VITE_SOURCE_EMITTER||'',
  destinationToken:env.VITE_DESTINATION_TOKEN||'',destinationRegistry:env.VITE_DESTINATION_REGISTRY||'',
  destinationSettlement:env.VITE_DESTINATION_SETTLEMENT||'',
  nearAdapterAccount:env.VITE_NEAR_ADAPTER_ACCOUNT||'b6739911d06797c8d55539944a5226d132c2001e34d7bf38993e735954b162d3'
};
export function validAddress(value){return !!value&&isAddress(value)&&value!==ZeroAddress;}
export function compact(value,n=6){return value?value.slice(0,n+2)+'…'+value.slice(-4):'—';}
