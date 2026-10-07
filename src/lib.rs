#![no_std]
//! The smallest useful consumer of a Sorogate access policy: a gate.
//!
//! `enter(subject)` lets an address in if the policy says it qualifies, and records that it did. It is about as short as a
//! consumer can be while doing the three things every consumer must do, in this order:
//!
//! 1. **Authenticate the subject.** `subject.require_auth()`. The policy contract's `evaluate` answers for any address it is
//!    given and does not prove who is asking. Without this line anyone could enter on behalf of any qualifying address.
//! 2. **Ask the policy contract, and fail closed.** If it cannot answer (wrong address, unknown policy, an error inside it),
//!    the answer is no, never yes.
//! 3. **Enforce the decision**, not merely read it: a denial stops the call, and nothing is recorded.
//!
//! Sorogate's interface is declared here by hand, as any consumer would declare it: this crate does not depend on Sorogate's
//! code. The tests run against the real policy contract, to keep the declaration honest.
//!
//! **This is an example, not a product.** It has no administrator and holds nothing of value.
use soroban_sdk::{
    contract, contractclient, contracterror, contractevent, contractimpl, contracttype, Address,
    Env,
};

#[cfg(test)]
mod test;

const DAY_IN_LEDGERS: u32 = 17_280;
/// A record of entry is kept for 90 days at a time. A forgotten one is archived, not erased.
const RECORD_TTL_EXTEND_TO: u32 = 90 * DAY_IN_LEDGERS;
const RECORD_TTL_THRESHOLD: u32 = 30 * DAY_IN_LEDGERS;

// ---------------------------------------------------------------- Sorogate's interface

/// Why a policy denied someone. The codes are Sorogate's, and are part of its interface.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum DenyReason {
    None = 0,
    Inactive = 1,
    BelowMinimum = 2,
    BalanceUnavailable = 3,
    BeforeWindow = 4,
    AfterWindow = 5,
}

/// What Sorogate's `evaluate` returns.
#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub allowed: bool,
    pub version: u32,
    pub failed_index: Option<u32>,
    pub reason: DenyReason,
}

#[contractclient(name = "PolicyClient")]
pub trait PolicyInterface {
    fn evaluate(env: Env, id: u64, subject: Address) -> Decision;
}

// ---------------------------------------------------------------- this contract

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The policy contract could not answer: wrong address, unknown policy id, or a failure inside it.
    PolicyUnavailable = 1,
    // Denials: the policy said no. These mirror `DenyReason`, so a caller can tell why.
    Inactive = 10,
    BelowMinimum = 11,
    BalanceUnavailable = 12,
    BeforeWindow = 13,
    AfterWindow = 14,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub policy_contract: Address,
    pub policy_id: u64,
}

#[contracttype]
enum DataKey {
    Config,
    Member(Address),
}

#[contractevent(topics = ["gate", "entered"])]
pub struct Entered {
    #[topic]
    pub subject: Address,
    /// The version of the policy the decision was made under.
    pub policy_version: u32,
}

#[contract]
pub struct Gate;

#[contractimpl]
impl Gate {
    /// Sets up the gate, once: which policy contract, and which policy in it.
    pub fn __constructor(env: Env, policy_contract: Address, policy_id: u64) {
        env.storage().instance().set(
            &DataKey::Config,
            &Config {
                policy_contract,
                policy_id,
            },
        );
    }

    /// Lets `subject` in, and records it, if the policy allows them.
    pub fn enter(env: Env, subject: Address) -> Result<(), Error> {
        // 1. Prove the caller is the subject. Everything below is about what the subject is entitled to.
        subject.require_auth();

        let config: Config = env.storage().instance().get(&DataKey::Config).unwrap();

        // 2. Ask the policy. Any failure to get an answer is a refusal.
        let decision = match PolicyClient::new(&env, &config.policy_contract)
            .try_evaluate(&config.policy_id, &subject)
        {
            Ok(Ok(decision)) => decision,
            _ => return Err(Error::PolicyUnavailable),
        };

        // 3. Enforce it.
        if !decision.allowed {
            return Err(match decision.reason {
                DenyReason::Inactive => Error::Inactive,
                DenyReason::BelowMinimum => Error::BelowMinimum,
                DenyReason::BalanceUnavailable => Error::BalanceUnavailable,
                DenyReason::BeforeWindow => Error::BeforeWindow,
                DenyReason::AfterWindow => Error::AfterWindow,
                // `allowed` false with reason `None` breaks Sorogate's own rule; do not let anyone in.
                DenyReason::None => Error::PolicyUnavailable,
            });
        }

        let key = DataKey::Member(subject.clone());
        env.storage()
            .persistent()
            .set(&key, &env.ledger().sequence());
        env.storage()
            .persistent()
            .extend_ttl(&key, RECORD_TTL_THRESHOLD, RECORD_TTL_EXTEND_TO);
        env.storage()
            .instance()
            .extend_ttl(RECORD_TTL_THRESHOLD, RECORD_TTL_EXTEND_TO);
        Entered {
            subject,
            policy_version: decision.version,
        }
        .publish(&env);
        Ok(())
    }

    /// Whether `subject` has been let in.
    pub fn is_member(env: Env, subject: Address) -> bool {
        env.storage().persistent().has(&DataKey::Member(subject))
    }

    /// What the gate was set up with.
    pub fn config(env: Env) -> Config {
        env.storage().instance().get(&DataKey::Config).unwrap()
    }
}
