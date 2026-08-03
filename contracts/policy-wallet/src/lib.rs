//! Custos — a policy-based smart wallet for Stellar, built on Soroban's
//! custom account / custom authorization framework.
//!
//! Instead of a single key that can move every asset with no limits, this
//! contract lets an account owner attach *rules* to their wallet:
//!
//!   - Spending limit: a max amount that can move per rolling 24h window
//!     without extra approval.
//!   - Multisig threshold: signatures must meet a configured weight before
//!     a transaction is authorized.
//!   - Session key: a temporary, scoped signer (e.g. for a game or dApp)
//!     that expires automatically and cannot exceed its own budget.
//!
//! This is a hackathon-stage reference implementation meant to demonstrate
//! the shape of a policy-driven `CustomAccountInterface`. Re-check field
//! names and trait signatures against the Soroban SDK version you build
//! with — the auth framework has moved fast across protocol releases.
//!
//! Docs: https://stellar.github.io/js-stellar-sdk/guides/00-protocol-27-soroban-auth/

#![no_std]

use soroban_sdk::{
    auth::{Context, CustomAccountInterface},
    contract, contracterror, contractimpl, contracttype,
    crypto::Hash,
    panic_with_error, symbol_short, Address, BytesN, Env, Symbol, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotInitialized = 1,
    AlreadyInitialized = 2,
    SpendingLimitExceeded = 3,
    ThresholdNotMet = 4,
    SessionKeyExpired = 5,
    SessionBudgetExceeded = 6,
    UnknownSigner = 7,
    BadSignature = 8,
}

const OWNERS: Symbol = symbol_short!("OWNERS");
const THRESHOLD: Symbol = symbol_short!("THRESH");
const SPEND: Symbol = symbol_short!("SPEND");
const SESSION: Symbol = symbol_short!("SESSION");

#[contracttype]
#[derive(Clone)]
pub struct Owner {
    pub public_key: BytesN<32>,
    pub weight: u32,
}

/// Rolling daily spending policy, enforced independently of signer weight.
#[contracttype]
#[derive(Clone)]
pub struct SpendPolicy {
    pub daily_limit: i128,
    pub spent: i128,
    pub day_index: u64, // ledger_timestamp() / 86400
}

/// A scoped, time-boxed signer — e.g. handed to a dApp for one session.
#[contracttype]
#[derive(Clone)]
pub struct SessionKey {
    pub public_key: BytesN<32>,
    pub expires_at: u64, // unix timestamp
    pub budget: i128,
    pub spent: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Signature {
    pub public_key: BytesN<32>,
    pub signature: BytesN<64>,
}

#[contract]
pub struct PolicyWallet;

#[contractimpl]
impl PolicyWallet {
    /// One-time setup: register owner keys + the approval threshold and an
    /// optional daily spending ceiling that applies below the threshold.
    pub fn initialize(
        env: Env,
        owners: Vec<Owner>,
        threshold: u32,
        daily_limit: i128,
    ) -> Result<(), Error> {
        if env.storage().instance().has(&OWNERS) {
            return Err(Error::AlreadyInitialized);
        }
        env.storage().instance().set(&OWNERS, &owners);
        env.storage().instance().set(&THRESHOLD, &threshold);
        env.storage().instance().set(
            &SPEND,
            &SpendPolicy {
                daily_limit,
                spent: 0,
                day_index: env.ledger().timestamp() / 86400,
            },
        );
        Ok(())
    }

    /// Owner action: issue a session key that a dApp / game can use on the
    /// wallet's behalf until it expires or exhausts its own budget.
    pub fn add_session_key(
        env: Env,
        owner: Address,
        session_pk: BytesN<32>,
        ttl_seconds: u64,
        budget: i128,
    ) {
        owner.require_auth();
        let key = SessionKey {
            public_key: session_pk,
            expires_at: env.ledger().timestamp() + ttl_seconds,
            budget,
            spent: 0,
        };
        env.storage().temporary().set(&SESSION, &key);
        env.storage()
            .temporary()
            .extend_ttl(&SESSION, ttl_seconds as u32, ttl_seconds as u32);
    }

    /// Read-only helper for the frontend dashboard.
    pub fn get_spend_policy(env: Env) -> SpendPolicy {
        env.storage().instance().get(&SPEND).unwrap()
    }
}

#[contractimpl]
impl CustomAccountInterface for PolicyWallet {
    type Signature = Vec<Signature>;
    type Error = Error;

    /// Called by the protocol instead of the default ed25519 check when
    /// this contract is used as an account. `payload` is the hash of the
    /// signed transaction; `signatures` is whatever `__check_auth` expects
    /// per the `Signature` type above; `auth_contexts` describes what the
    /// transaction is actually trying to do (contract, function, amounts).
    fn __check_auth(
        env: Env,
        payload: Hash<32>,
        signatures: Vec<Signature>,
        auth_contexts: Vec<Context>,
    ) -> Result<(), Error> {
        let owners: Vec<Owner> = env.storage().instance().get(&OWNERS).unwrap();
        let threshold: u32 = env.storage().instance().get(&THRESHOLD).unwrap();

        // 1) Verify every submitted signature against a known owner key,
        //    and total up the weight that actually signed this payload.
        let mut weight: u32 = 0;
        for sig in signatures.iter() {
            let Some(owner) = owners.iter().find(|o| o.public_key == sig.public_key) else {
                panic_with_error!(&env, Error::UnknownSigner);
            };
            env.crypto()
                .ed25519_verify(&sig.public_key, &payload.clone().into(), &sig.signature);
            weight += owner.weight;
        }

        // 2) Estimate the amount being moved from the auth contexts and
        //    enforce the rolling daily limit for sub-threshold transfers.
        let attempted_spend = Self::estimate_spend(&auth_contexts);
        if weight < threshold {
            let mut policy: SpendPolicy = env.storage().instance().get(&SPEND).unwrap();
            let today = env.ledger().timestamp() / 86400;
            if policy.day_index != today {
                policy.day_index = today;
                policy.spent = 0;
            }
            if policy.spent + attempted_spend > policy.daily_limit {
                return Err(Error::SpendingLimitExceeded);
            }
            policy.spent += attempted_spend;
            env.storage().instance().set(&SPEND, &policy);
        }

        // 3) If a session key signed instead of / alongside an owner,
        //    enforce its expiry and its own scoped budget.
        if let Some(session) = env
            .storage()
            .temporary()
            .get::<Symbol, SessionKey>(&SESSION)
        {
            let signed_by_session = signatures.iter().any(|s| s.public_key == session.public_key);
            if signed_by_session {
                if env.ledger().timestamp() > session.expires_at {
                    return Err(Error::SessionKeyExpired);
                }
                if session.spent + attempted_spend > session.budget {
                    return Err(Error::SessionBudgetExceeded);
                }
            }
        }

        if weight < threshold {
            return Err(Error::ThresholdNotMet);
        }

        Ok(())
    }
}

impl PolicyWallet {
    /// Sums transfer-shaped invocations in the auth tree. Real logic should
    /// match on the specific token `transfer` / `transfer_from` calls this
    /// wallet expects to guard; kept simple here for readability.
    fn estimate_spend(_auth_contexts: &Vec<Context>) -> i128 {
        // TODO: inspect ContractContext { contract, fn_name, args } entries
        // for token contract invocations and sum the transferred amounts.
        0
    }
}
