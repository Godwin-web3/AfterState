import { getBytes, hexlify } from 'ethers';

export const NEAR_RPC = 'https://rpc.testnet.near.org';

export async function nearRpc(method, params) {
  const response = await fetch(NEAR_RPC, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', id: 'afterstate', method, params })
  });
  if (!response.ok) throw new Error('NEAR RPC HTTP ' + response.status);
  const body = await response.json();
  if (body.error) throw new Error(body.error.message || 'NEAR RPC request failed');
  return body.result;
}

export async function readAdapterConfig(accountId) {
  const argsBase64 = btoa('{}');
  const result = await nearRpc('query', {
    request_type: 'call_function',
    finality: 'final',
    account_id: accountId,
    method_name: 'config',
    args_base64: argsBase64
  });
  return JSON.parse(new TextDecoder().decode(new Uint8Array(result.result)));
}

export function fixedBytes(value, length) {
  const bytes = typeof value === 'string' ? getBytes(value) : Uint8Array.from(value);
  if (bytes.length !== length) throw new Error('Expected exactly ' + length + ' bytes.');
  return Array.from(bytes);
}

export function bytesToHex(value) {
  return hexlify(Uint8Array.from(value));
}

export function extractNearTxHash(result) {
  return result?.transaction?.hash
    || result?.transaction_outcome?.id
    || result?.transactionOutcome?.id
    || result?.transactionHash
    || result?.txHash
    || null;
}

export async function waitForNearOutcome(txHash, signerId, timeoutMs = 180000) {
  const started = Date.now();
  let lastError;
  while (Date.now() - started < timeoutMs) {
    try {
      const outcome = await nearRpc('EXPERIMENTAL_tx_status', [txHash, signerId]);
      const receipts = outcome.receipts_outcome || [];
      const failed = receipts.find(item => item.outcome?.status?.Failure);
      if (outcome.status?.Failure || failed) {
        const reason = outcome.status?.Failure || failed?.outcome?.status?.Failure;
        throw new Error('NEAR adapter execution failed: ' + JSON.stringify(reason));
      }
      const finalReceiptStatus = receipts.slice().reverse()
        .map(item => item.outcome?.status)
        .find(status => status && (status.Failure || Object.prototype.hasOwnProperty.call(status, 'SuccessValue')));
      if (finalReceiptStatus?.SuccessValue && finalReceiptStatus.SuccessValue.length > 0) return outcome;
    } catch (error) {
      if (/execution failed/i.test(error.message)) throw error;
      lastError = error;
    }
    await new Promise(resolve => setTimeout(resolve, 2500));
  }
  throw new Error('Timed out waiting for NEAR MPC execution.' + (lastError ? ' Last RPC response: ' + lastError.message : ''));
}

export function extractEvmSignature(outcome) {
  const statuses = [
    ...(outcome.receipts_outcome || []).slice().reverse().map(item => item.outcome?.status),
    outcome.status
  ];
  for (const status of statuses) {
    if (!status || !Object.prototype.hasOwnProperty.call(status, 'SuccessValue')) continue;
    const encoded = status.SuccessValue;
    if (!encoded) continue;
    const raw = Uint8Array.from(atob(encoded), char => char.charCodeAt(0));
    const text = new TextDecoder().decode(raw);
    try {
      const parsed = JSON.parse(text);
      if (Array.isArray(parsed)) {
        const signature = Uint8Array.from(parsed);
        if (signature.length === 65) return hexlify(signature);
      }
      if (typeof parsed === 'string' && /^0x[0-9a-fA-F]{130}$/.test(parsed)) return parsed;
    } catch {
      if (raw.length === 65) return hexlify(raw);
    }
  }
  throw new Error('NEAR completed without returning the expected 65-byte EVM signature. Inspect the final receipt outcome before retrying.');
}
