use afterstate_near_foreign_tx_fixtures::{
    build_base_foreign_tx_request, BaseEvmLogCandidate,
};
use near_mpc_contract_interface::types::{
    DomainId, Hash256, VerifyForeignTransactionResponse,
};
use near_sdk::{
    env, ext_contract, near, require, AccountId, Gas, NearToken, Promise, PromiseError,
};

#[ext_contract(ext_mpc)]
trait ExtMpc {
    fn verify_foreign_transaction(
        &mut self,
        request: near_mpc_contract_interface::types::VerifyForeignTransactionRequestArgs,
    ) -> VerifyForeignTransactionResponse;
}

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
/// This contract deliberately does not claim to produce an AfterState EIP-712
/// signature yet. It submits a candidate Base log with the exact MPC 3.19.0
/// expected payload hash and returns only after the MPC contract accepts the
/// response and the callback re-checks that hash.
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct AfterStateNearAdapter {
    mpc_contract_id: AccountId,
    authorized_submitter: AccountId,
    foreign_tx_domain_id: DomainId,
    mpc_call_gas_tgas: u64,
    callback_gas_tgas: u64,
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
    ) -> Self {
        require!(mpc_call_gas_tgas > 0, "MPC call gas must be non-zero");
        require!(callback_gas_tgas > 0, "callback gas must be non-zero");
        Self {
            mpc_contract_id,
            authorized_submitter,
            foreign_tx_domain_id: DomainId(foreign_tx_domain_id),
            mpc_call_gas_tgas,
            callback_gas_tgas,
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

        let request = build_base_foreign_tx_request(
            candidate,
            self.foreign_tx_domain_id,
        )
        .unwrap_or_else(|_| env::panic_str("removed candidate log"));

        let expected_payload_hash = request
            .expected_payload_hash
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

    #[private]
    pub fn on_foreign_tx_verified(
        &self,
        expected_payload_hash: Hash256,
        #[callback_result] result: Result<VerifyForeignTransactionResponse, PromiseError>,
    ) -> VerifyForeignTransactionResponse {
        let response = result.unwrap_or_else(|_| {
            env::panic_str("NEAR MPC ForeignTx verification failed")
        });
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
}
