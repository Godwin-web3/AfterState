use afterstate_near_foreign_tx_fixtures::{
    build_base_foreign_tx_request, decode_source_payment, BaseEvmLogCandidate,
};
use near_mpc_contract_interface::types::{
    DomainId, Hash256, VerifyForeignTransactionResponse,
};
use near_sdk::{
    env, ext_contract, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise,
    PromiseError,
};

#[allow(dead_code)]
#[ext_contract(ext_mpc)]
trait ExtMpc {
    fn verify_foreign_transaction(
        &mut self,
        request: near_mpc_contract_interface::types::VerifyForeignTransactionRequestArgs,
    ) -> VerifyForeignTransactionResponse;
}

#[allow(dead_code)]
#[ext_contract(ext_self)]
trait ExtSelf {
    fn on_foreign_tx_verified(
        &self,
        expected_payload_hash: Hash256,
        #[callback_result] result: Result<VerifyForeignTransactionResponse, PromiseError>,
    ) -> VerifyForeignTransactionResponse;
}

/// Minimal NEAR adapter for the MPC ForeignTx verification stage.
///
/// The configured agreement is immutable in this first vertical slice. The
/// adapter validates the SourcePayment event schema and agreement fields
/// before asking MPC to independently verify the exact source log.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct AfterStateNearAdapter {
    mpc_contract_id: AccountId,
    authorized_submitter: AccountId,
    foreign_tx_domain_id: DomainId,
    mpc_call_gas_tgas: u64,
    callback_gas_tgas: u64,
    source_emitter: [u8; 20],
    agreement_id: [u8; 32],
    agreement_version: u32,
    asset: [u8; 20],
    recipient: [u8; 20],
    min_amount: u128,
    max_amount: u128,
    agreement_deadline: u64,
    finality_policy_id: [u8; 32],
}

#[near]
impl AfterStateNearAdapter {
    #[init]
    pub fn new(
        mpc_contract_id: AccountId,
        authorized_submitter: AccountId,
        foreign_tx_domain_id: u64,
        mpc_call_gas_tgas: u64,
        callback_gas_tgas: u64,
        source_emitter: [u8; 20],
        agreement_id: [u8; 32],
        agreement_version: u32,
        asset: [u8; 20],
        recipient: [u8; 20],
        min_amount: u128,
        max_amount: u128,
        agreement_deadline: u64,
        finality_policy_id: [u8; 32],
    ) -> Self {
        require!(mpc_call_gas_tgas > 0, "MPC call gas must be non-zero");
        require!(callback_gas_tgas > 0, "callback gas must be non-zero");
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

        Self {
            mpc_contract_id,
            authorized_submitter,
            foreign_tx_domain_id: DomainId(foreign_tx_domain_id),
            mpc_call_gas_tgas,
            callback_gas_tgas,
            source_emitter,
            agreement_id,
            agreement_version,
            asset,
            recipient,
            min_amount,
            max_amount,
            agreement_deadline,
            finality_policy_id,
        }
    }

    /// Submit a candidate Base EVM log for MPC verification.
    ///
    /// Candidate fields are untrusted. The MPC contract independently checks
    /// the transaction and extracted log against the expected payload hash.
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
        require!(
            payment.source_timestamp <= self.agreement_deadline,
            "source payment is after agreement deadline"
        );
        require!(
            candidate.transaction_hash != [0; 32]
                && candidate.block_hash != [0; 32]
                && candidate.block_number > 0,
            "incomplete source log identity"
        );

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
                    .with_static_gas(Gas::from_tgas(self.callback_gas_tgas))
                    .on_foreign_tx_verified(expected_payload_hash),
            )
    }

    pub fn on_foreign_tx_verified(
        &self,
        expected_payload_hash: Hash256,
        #[callback_result] result: Result<VerifyForeignTransactionResponse, PromiseError>,
    ) -> VerifyForeignTransactionResponse {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            "private callback"
        );
        let response = result
            .unwrap_or_else(|_| env::panic_str("NEAR MPC ForeignTx verification failed"));
        require!(
            response.payload_hash == expected_payload_hash,
            "MPC returned unexpected payload hash"
        );
        response
    }

    pub fn config(&self) -> AdapterConfig {
        AdapterConfig {
            mpc_contract_id: self.mpc_contract_id.clone(),
            authorized_submitter: self.authorized_submitter.clone(),
            foreign_tx_domain_id: self.foreign_tx_domain_id,
            mpc_call_gas_tgas: self.mpc_call_gas_tgas,
            callback_gas_tgas: self.callback_gas_tgas,
            source_emitter: self.source_emitter,
            agreement_id: self.agreement_id,
            agreement_version: self.agreement_version,
            asset: self.asset,
            recipient: self.recipient,
            min_amount: self.min_amount,
            max_amount: self.max_amount,
            agreement_deadline: self.agreement_deadline,
            finality_policy_id: self.finality_policy_id,
        }
    }
}

#[near(serializers = [json])]
#[derive(Clone)]
pub struct AdapterConfig {
    pub mpc_contract_id: AccountId,
    pub authorized_submitter: AccountId,
    pub foreign_tx_domain_id: DomainId,
    pub mpc_call_gas_tgas: u64,
    pub callback_gas_tgas: u64,
    pub source_emitter: [u8; 20],
    pub agreement_id: [u8; 32],
    pub agreement_version: u32,
    pub asset: [u8; 20],
    pub recipient: [u8; 20],
    pub min_amount: u128,
    pub max_amount: u128,
    pub agreement_deadline: u64,
    pub finality_policy_id: [u8; 32],
}
