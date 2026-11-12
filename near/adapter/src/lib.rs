use afterstate_near_foreign_tx_fixtures::{
    afterstate_eip712_digest, build_base_foreign_tx_request, decode_source_payment,
    mpc_signature_to_evm_bytes, source_payment_topic0, BaseEvmLogCandidate, Eip712SourceFact,
};
use near_mpc_contract_interface::types::{
    DomainId, Hash256, Payload, SignRequestArgs, SignatureResponse,
    VerifyForeignTransactionRequestArgs, VerifyForeignTransactionResponse,
};
use near_sdk::{
    env, ext_contract, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise,
    PromiseError,
};

const BASE_SEPOLIA_CHAIN_ID: u64 = 84_532;
const SIGN_PATH: &str = "afterstate-v1";
// Live MPC testnet config v3.18.0 requires at least 15 Tgas for both
// verify_foreign_transaction and sign calls.
const MIN_MPC_REQUEST_GAS_TGAS: u64 = 15;

#[allow(dead_code)]
#[ext_contract(ext_mpc)]
trait ExtMpc {
    fn verify_foreign_transaction(
        &mut self,
        request: VerifyForeignTransactionRequestArgs,
    ) -> VerifyForeignTransactionResponse;

    fn sign(&mut self, request: SignRequestArgs) -> SignatureResponse;
}

#[allow(dead_code)]
#[ext_contract(ext_self)]
trait ExtSelf {
    fn on_foreign_tx_verified(
        &self,
        expected_payload_hash: Hash256,
        fact: Eip712SourceFact,
        #[callback_result] result: Result<VerifyForeignTransactionResponse, PromiseError>,
    ) -> Promise;

    fn on_signature(
        &self,
        #[callback_result] result: Result<SignatureResponse, PromiseError>,
    ) -> Vec<u8>;
}

/// NEAR adapter for Base ForeignTx verification followed by a fixed-path
/// Chain Signatures EIP-712 signature. The configured agreement is immutable
/// in this first vertical slice.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct AfterStateNearAdapter {
    mpc_contract_id: AccountId,
    authorized_submitter: AccountId,
    foreign_tx_domain_id: DomainId,
    sign_domain_id: DomainId,
    mpc_call_gas_tgas: u64,
    foreign_callback_gas_tgas: u64,
    sign_call_gas_tgas: u64,
    signature_callback_gas_tgas: u64,
    source_emitter: [u8; 20],
    agreement_id: [u8; 32],
    agreement_version: u32,
    asset: [u8; 20],
    recipient: [u8; 20],
    min_amount: u128,
    max_amount: u128,
    agreement_deadline: u64,
    finality_policy_id: [u8; 32],
    destination_chain_id: u64,
    verifier_contract: [u8; 20],
    destination_registry: [u8; 20],
    signer_version: u32,
}

#[near]
impl AfterStateNearAdapter {
    #[init]
    pub fn new(
        mpc_contract_id: AccountId,
        authorized_submitter: AccountId,
        foreign_tx_domain_id: u64,
        sign_domain_id: u64,
        mpc_call_gas_tgas: u64,
        foreign_callback_gas_tgas: u64,
        sign_call_gas_tgas: u64,
        signature_callback_gas_tgas: u64,
        source_emitter: [u8; 20],
        agreement_id: [u8; 32],
        agreement_version: u32,
        asset: [u8; 20],
        recipient: [u8; 20],
        min_amount: u128,
        max_amount: u128,
        agreement_deadline: u64,
        finality_policy_id: [u8; 32],
        destination_chain_id: u64,
        verifier_contract: [u8; 20],
        destination_registry: [u8; 20],
        signer_version: u32,
    ) -> Self {
        // ForeignTx and Sign are separate MPC trust domains. Reusing one domain
        // would erase the protocol-level separation between observation and
        // destination authorization.
        require!(foreign_tx_domain_id != sign_domain_id, "ForeignTx and Sign domains must be distinct");
        require!(mpc_contract_id != env::current_account_id(), "MPC contract cannot be the adapter itself");
        require!(
            mpc_call_gas_tgas >= MIN_MPC_REQUEST_GAS_TGAS,
            "MPC ForeignTx call gas is below the live contract minimum"
        );
        require!(
            foreign_callback_gas_tgas > sign_call_gas_tgas
                && foreign_callback_gas_tgas - sign_call_gas_tgas > signature_callback_gas_tgas,
            "ForeignTx callback gas must cover the follow-on Sign call and signature callback"
        );
        require!(
            sign_call_gas_tgas >= MIN_MPC_REQUEST_GAS_TGAS,
            "MPC Sign call gas is below the live contract minimum"
        );
        require!(signature_callback_gas_tgas > 0, "signature callback gas must be non-zero");
        require!(source_emitter != [0; 20], "source emitter is zero");
        require!(agreement_id != [0; 32], "agreement ID is zero");
        require!(agreement_version > 0, "agreement version is zero");
        require!(asset != [0; 20], "asset is zero");
        require!(recipient != [0; 20], "recipient is zero");
        require!(min_amount > 0 && max_amount >= min_amount, "invalid amount bounds");
        require!(
            agreement_deadline > env::block_timestamp() / 1_000_000_000,
            "agreement deadline has passed"
        );
        require!(finality_policy_id != [0; 32], "finality policy is zero");
        require!(destination_chain_id > 0, "destination chain ID is zero");
        require!(verifier_contract != [0; 20], "verifier contract is zero");
        require!(destination_registry != [0; 20], "destination registry is zero");
        require!(signer_version > 0, "signer version is zero");

        Self {
            mpc_contract_id,
            authorized_submitter,
            foreign_tx_domain_id: DomainId(foreign_tx_domain_id),
            sign_domain_id: DomainId(sign_domain_id),
            mpc_call_gas_tgas,
            foreign_callback_gas_tgas,
            sign_call_gas_tgas,
            signature_callback_gas_tgas,
            source_emitter,
            agreement_id,
            agreement_version,
            asset,
            recipient,
            min_amount,
            max_amount,
            agreement_deadline,
            finality_policy_id,
            destination_chain_id,
            verifier_contract,
            destination_registry,
            signer_version,
        }
    }

    /// Validate a SourcePayment candidate, ask MPC to verify its exact log,
    /// then ask the separate Sign-purpose domain to sign the AfterState digest.
    pub fn verify_base_log(&self, candidate: BaseEvmLogCandidate) -> Promise {
        require!(
            env::predecessor_account_id() == self.authorized_submitter,
            "unauthorized submitter"
        );

        let payment = decode_source_payment(&candidate)
            .unwrap_or_else(|_| env::panic_str("invalid SourcePayment event"));

        require!(candidate.emitter == self.source_emitter, "wrong source emitter");
        require!(payment.agreement_id == self.agreement_id, "wrong agreement ID");
        require!(payment.agreement_version == self.agreement_version, "wrong agreement version");
        require!(payment.asset == self.asset, "wrong source asset");
        require!(payment.recipient == self.recipient, "wrong recipient");
        require!(
            payment.amount >= self.min_amount && payment.amount <= self.max_amount,
            "amount outside agreement bounds"
        );
        let now_seconds = env::block_timestamp() / 1_000_000_000;
        require!(now_seconds <= self.agreement_deadline, "agreement deadline has passed");
        require!(
            payment.source_timestamp <= now_seconds,
            "source event timestamp is in the future"
        );
        // The EVM registry requires expiresAt > observedAt. Since expiresAt is
        // the agreement deadline, a payment timestamp equal to the deadline
        // would produce a fact that can never be consumed.
        require!(
            payment.source_timestamp < self.agreement_deadline,
            "source payment must precede agreement deadline"
        );
        require!(
            candidate.transaction_hash != [0; 32]
                && candidate.block_hash != [0; 32]
                && candidate.block_number > 0,
            "incomplete source log identity"
        );

        let fact = Eip712SourceFact {
            source_chain_id: BASE_SEPOLIA_CHAIN_ID,
            emitter: candidate.emitter,
            event_schema: source_payment_topic0(),
            agreement_id: payment.agreement_id,
            source_block_hash: candidate.block_hash,
            source_tx_hash: candidate.transaction_hash,
            source_block_number: candidate.block_number,
            log_index: candidate.log_index,
            asset: payment.asset,
            amount: payment.amount,
            recipient: payment.recipient,
            // This is the source event's block timestamp, authenticated in event data.
            observed_at: payment.source_timestamp,
            expires_at: self.agreement_deadline,
            key_version: self.signer_version,
            agreement_version: payment.agreement_version,
            finality_policy_id: self.finality_policy_id,
            nonce: payment.nonce,
        };

        let request = build_base_foreign_tx_request(candidate, self.foreign_tx_domain_id)
            .unwrap_or_else(|_| env::panic_str("removed candidate log"));
        let expected_payload_hash = request
            .expected_payload_hash
            .clone()
            .expect("builder always sets expected_payload_hash");

        ext_mpc::ext(self.mpc_contract_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(self.mpc_call_gas_tgas))
            .verify_foreign_transaction(request)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(self.foreign_callback_gas_tgas))
                    .on_foreign_tx_verified(expected_payload_hash, fact),
            )
    }

    pub fn on_foreign_tx_verified(
        &self,
        expected_payload_hash: Hash256,
        fact: Eip712SourceFact,
        #[callback_result] result: Result<VerifyForeignTransactionResponse, PromiseError>,
    ) -> Promise {
        self.assert_private_callback();
        let response = result
            .unwrap_or_else(|_| env::panic_str("NEAR MPC ForeignTx verification failed"));
        require!(
            response.payload_hash == expected_payload_hash,
            "MPC returned unexpected payload hash"
        );

        let digest = afterstate_eip712_digest(
            &fact,
            self.destination_chain_id,
            self.verifier_contract,
            self.destination_registry,
        );
        let sign_request = SignRequestArgs {
            path: SIGN_PATH.to_string(),
            payload: Payload::from_legacy_ecdsa(digest),
            domain_id: self.sign_domain_id,
        };

        ext_mpc::ext(self.mpc_contract_id.clone())
            .with_attached_deposit(NearToken::from_yoctonear(1))
            .with_static_gas(Gas::from_tgas(self.sign_call_gas_tgas))
            .sign(sign_request)
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(self.signature_callback_gas_tgas))
                    .on_signature(),
            )
    }

    pub fn on_signature(
        &self,
        #[callback_result] result: Result<SignatureResponse, PromiseError>,
    ) -> Vec<u8> {
        self.assert_private_callback();
        let response = result.unwrap_or_else(|_| env::panic_str("Chain Signatures signing failed"));
        mpc_signature_to_evm_bytes(response)
            .unwrap_or_else(|_| env::panic_str("MPC signature is not a valid EVM signature"))
            .to_vec()
    }

    fn assert_private_callback(&self) {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            "private callback"
        );
    }

    pub fn config(&self) -> AdapterConfig {
        AdapterConfig {
            mpc_contract_id: self.mpc_contract_id.clone(),
            authorized_submitter: self.authorized_submitter.clone(),
            foreign_tx_domain_id: self.foreign_tx_domain_id,
            sign_domain_id: self.sign_domain_id,
            mpc_call_gas_tgas: self.mpc_call_gas_tgas,
            foreign_callback_gas_tgas: self.foreign_callback_gas_tgas,
            sign_call_gas_tgas: self.sign_call_gas_tgas,
            signature_callback_gas_tgas: self.signature_callback_gas_tgas,
            source_emitter: self.source_emitter,
            agreement_id: self.agreement_id,
            agreement_version: self.agreement_version,
            asset: self.asset,
            recipient: self.recipient,
            min_amount: self.min_amount,
            max_amount: self.max_amount,
            agreement_deadline: self.agreement_deadline,
            finality_policy_id: self.finality_policy_id,
            destination_chain_id: self.destination_chain_id,
            verifier_contract: self.verifier_contract,
            destination_registry: self.destination_registry,
            signer_version: self.signer_version,
        }
    }
}

#[near(serializers = [json])]
#[derive(Clone)]
pub struct AdapterConfig {
    pub mpc_contract_id: AccountId,
    pub authorized_submitter: AccountId,
    pub foreign_tx_domain_id: DomainId,
    pub sign_domain_id: DomainId,
    pub mpc_call_gas_tgas: u64,
    pub foreign_callback_gas_tgas: u64,
    pub sign_call_gas_tgas: u64,
    pub signature_callback_gas_tgas: u64,
    pub source_emitter: [u8; 20],
    pub agreement_id: [u8; 32],
    pub agreement_version: u32,
    pub asset: [u8; 20],
    pub recipient: [u8; 20],
    pub min_amount: u128,
    pub max_amount: u128,
    pub agreement_deadline: u64,
    pub finality_policy_id: [u8; 32],
    pub destination_chain_id: u64,
    pub verifier_contract: [u8; 20],
    pub destination_registry: [u8; 20],
    pub signer_version: u32,
}


#[cfg(test)]
mod adapter_tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::str::FromStr;

    fn construct(foreign_domain: u64, sign_domain: u64) -> AfterStateNearAdapter {
        construct_with_gas(foreign_domain, sign_domain, 20, 60, 20, 30)
    }

    fn construct_with_gas(
        foreign_domain: u64,
        sign_domain: u64,
        foreign_call_gas: u64,
        foreign_callback_gas: u64,
        sign_call_gas: u64,
        signature_callback_gas: u64,
    ) -> AfterStateNearAdapter {
        testing_env!(
            VMContextBuilder::new()
                .current_account_id(AccountId::from_str("adapter.testnet").unwrap())
                .block_timestamp(1_700_000_000_000_000_000)
                .build()
        );

        AfterStateNearAdapter::new(
            AccountId::from_str("mpc.testnet").unwrap(),
            AccountId::from_str("submitter.testnet").unwrap(),
            foreign_domain,
            sign_domain,
            foreign_call_gas,
            foreign_callback_gas,
            sign_call_gas,
            signature_callback_gas,
            [1; 20],
            [2; 32],
            1,
            [3; 20],
            [4; 20],
            1,
            100,
            1_800_000_000,
            [5; 32],
            BASE_SEPOLIA_CHAIN_ID,
            [6; 20],
            [7; 20],
            1,
        )
    }

    #[test]
    fn constructor_accepts_distinct_mpc_purposes() {
        let adapter = construct(10, 11);
        assert_eq!(adapter.foreign_tx_domain_id, DomainId(10));
        assert_eq!(adapter.sign_domain_id, DomainId(11));
    }

    #[test]
    fn constructor_rejects_reusing_foreign_tx_domain_for_signing() {
        let result = catch_unwind(AssertUnwindSafe(|| construct(10, 10)));
        assert!(result.is_err(), "same MPC domain was accepted for both purposes");
    }

    #[test]
    fn constructor_rejects_foreign_tx_gas_below_live_mpc_minimum() {
        let result = catch_unwind(AssertUnwindSafe(|| construct_with_gas(10, 11, 14, 60, 20, 30)));
        assert!(result.is_err(), "underfunded ForeignTx MPC call was accepted");
    }

    #[test]
    fn constructor_rejects_sign_gas_below_live_mpc_minimum() {
        let result = catch_unwind(AssertUnwindSafe(|| construct_with_gas(10, 11, 20, 60, 14, 30)));
        assert!(result.is_err(), "underfunded Sign MPC call was accepted");
    }

    #[test]
    fn constructor_rejects_foreign_callback_that_cannot_schedule_sign_promises() {
        let result = catch_unwind(AssertUnwindSafe(|| construct_with_gas(10, 11, 20, 50, 20, 30)));
        assert!(result.is_err(), "callback gas budget cannot cover follow-on promises");
    }
}
