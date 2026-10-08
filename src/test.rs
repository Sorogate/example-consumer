extern crate std;

use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Bytes, Env, IntoVal,
};

use crate::{DenyReason, Error, Gate, GateClient};

/// Sorogate's policy contract, as the network holds it: `tests/fixtures/access_policy.wasm` is the code fetched from the
/// public Testnet deployment (see the README). Importing it gives the real contract and its real types.
mod policy {
    soroban_sdk::contractimport!(file = "tests/fixtures/access_policy.wasm");
}

/// Contracts that stand where a policy contract should be and cannot, or will not, answer properly. The gate must treat each as
/// a refusal.
mod stand_ins {
    use soroban_sdk::{contract, contractimpl, Address, Env};

    use crate::{Decision, DenyReason};

    /// Fails inside `evaluate`.
    #[contract]
    pub struct Panics;
    #[contractimpl]
    impl Panics {
        pub fn evaluate(_env: Env, _id: u64, _subject: Address) -> Decision {
            panic!("this policy contract cannot answer")
        }
    }

    /// Answers with something that is not a `Decision`.
    #[contract]
    pub struct WrongType;
    #[contractimpl]
    impl WrongType {
        pub fn evaluate(_env: Env, _id: u64, _subject: Address) -> u32 {
            1
        }
    }

    /// Says no without a reason, which Sorogate's own rules forbid.
    #[contract]
    pub struct SaysNoWithoutAReason;
    #[contractimpl]
    impl SaysNoWithoutAReason {
        pub fn evaluate(_env: Env, _id: u64, _subject: Address) -> Decision {
            Decision {
                allowed: false,
                version: 1,
                failed_index: None,
                reason: DenyReason::None,
            }
        }
    }
}

struct Setup {
    env: Env,
    gate: GateClient<'static>,
    policy: policy::Client<'static>,
    policy_id: u64,
    token: Address,
}

fn policy_of(env: &Env, token: &Address, min: i128) -> soroban_sdk::Vec<policy::Condition> {
    vec![
        env,
        policy::Condition::TokenBalance(policy::TokenBalanceCond {
            token: token.clone(),
            min,
        }),
    ]
}

/// A policy "hold at least `min` of a token", a gate that uses it, and the token (a real Stellar asset contract).
fn setup(min: i128) -> Setup {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let policy_address = env.register(policy::WASM, ());
    let policy = policy::Client::new(&env, &policy_address);
    let policy_id = policy.create(&owner, &policy_of(&env, &token, min));
    let gate_address = env.register(Gate, (policy_address, policy_id));
    Setup {
        gate: GateClient::new(&env, &gate_address),
        policy,
        policy_id,
        token,
        env,
    }
}

fn give(s: &Setup, who: &Address, amount: i128) {
    StellarAssetClient::new(&s.env, &s.token).mint(who, &amount);
}

/// A second gate in the same environment, in front of whatever contract and policy id are given.
fn gate_in_front_of(s: &Setup, policy_contract: &Address, policy_id: u64) -> GateClient<'static> {
    let address = s.env.register(Gate, (policy_contract.clone(), policy_id));
    GateClient::new(&s.env, &address)
}

/// Makes `signer` the only address that has authorized anything: a call of `enter(subject)` on the gate.
fn only_signed_by(s: &Setup, signer: &Address, subject: &Address) {
    s.env.mock_auths(&[MockAuth {
        address: signer,
        invoke: &MockAuthInvoke {
            contract: &s.gate.address,
            fn_name: "enter",
            args: (subject.clone(),).into_val(&s.env),
            sub_invokes: &[],
        },
    }]);
}

// ------------------------------------------------------------------------------------------------ the pinned policy contract

#[test]
fn the_fixture_is_the_deployed_policy_contract() {
    // The same pin that `scripts/verify-deployment.sh` checks against the network.
    let pinned = include_str!("../tests/fixtures/access_policy.wasm.sha256")
        .split_whitespace()
        .next()
        .unwrap();
    let env = Env::default();
    let hash = env
        .crypto()
        .sha256(&Bytes::from_slice(&env, policy::WASM))
        .to_array();
    let hex: std::string::String = hash.iter().map(|b| std::format!("{b:02x}")).collect();
    assert_eq!(hex, pinned);
}

// ------------------------------------------------------------------------------------------------ a qualifying subject

#[test]
fn an_address_that_qualifies_and_authorizes_is_let_in_and_recorded() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    assert!(!s.gate.is_member(&alice));
    s.gate.enter(&alice);
    // The gate asked for alice's own authorization, not just anybody's. (Read straight after the call: the next call replaces it.)
    assert!(s.env.auths().iter().any(|(who, _)| *who == alice));
    assert!(s.gate.is_member(&alice));
}

// ------------------------------------------------------------------------------------------------ a subject that does not qualify

#[test]
fn an_address_below_the_minimum_is_refused_and_nothing_is_recorded() {
    let s = setup(100);
    let bob = Address::generate(&s.env);
    give(&s, &bob, 99);
    assert_eq!(s.gate.try_enter(&bob), Err(Ok(Error::BelowMinimum)));
    assert!(!s.gate.is_member(&bob));
}

#[test]
fn an_address_that_holds_nothing_is_refused() {
    let s = setup(100);
    let carol = Address::generate(&s.env);
    let refused = s.gate.try_enter(&carol);
    assert!(
        matches!(
            refused,
            Err(Ok(Error::BelowMinimum)) | Err(Ok(Error::BalanceUnavailable))
        ),
        "got {refused:?}"
    );
    assert!(!s.gate.is_member(&carol));
}

#[test]
fn a_deactivated_policy_lets_nobody_in() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 1_000);
    s.policy.set_active(&s.policy_id, &false);
    assert_eq!(s.gate.try_enter(&alice), Err(Ok(Error::Inactive)));
    assert!(!s.gate.is_member(&alice));
}

// ------------------------------------------------------------------------------------------------ authentication is the gate's job

#[test]
fn nobody_can_enter_on_a_qualifying_addresss_behalf() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    let bob = Address::generate(&s.env);
    give(&s, &alice, 100);

    // Alice qualifies. Bob, and only Bob, authorizes a call of `enter(alice)`. The policy would say yes; the gate must say no.
    only_signed_by(&s, &bob, &alice);
    let refused = s.gate.try_enter(&alice);
    assert!(
        matches!(refused, Err(Err(_))),
        "expected an authorization failure, got {refused:?}"
    );
    assert!(!s.gate.is_member(&alice));

    // The same call, authorized by alice herself, is accepted: so it was who signed that mattered.
    only_signed_by(&s, &alice, &alice);
    s.gate.enter(&alice);
    assert!(s.gate.is_member(&alice));
}

#[test]
fn with_no_authorization_at_all_a_qualifying_address_is_still_refused() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    s.env.set_auths(&[]);
    let refused = s.gate.try_enter(&alice);
    assert!(
        matches!(refused, Err(Err(_))),
        "expected an authorization failure, got {refused:?}"
    );
    assert!(!s.gate.is_member(&alice));
}

#[test]
fn the_subject_is_authenticated_before_the_policy_is_asked() {
    let s = setup(100);
    let bob = Address::generate(&s.env);
    give(&s, &bob, 1);
    // Bob does not qualify and has authorized nothing. The refusal must be about authorization, not about the policy: a caller
    // who cannot prove who they are learns nothing about what the policy would say.
    s.env.set_auths(&[]);
    let refused = s.gate.try_enter(&bob);
    assert!(
        matches!(refused, Err(Err(_))),
        "expected an authorization failure, got {refused:?}"
    );
}

// ------------------------------------------------------------------------------------------------ the policy owner changes the rule

#[test]
fn raising_the_minimum_refuses_a_subject_who_was_let_in_before() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 150);
    s.gate.enter(&alice);

    // The same gate, the same address, the same balance: the policy owner raises the minimum, with no redeploy.
    s.policy
        .update(&s.policy_id, &policy_of(&s.env, &s.token, 1_000));
    assert_eq!(s.gate.try_enter(&alice), Err(Ok(Error::BelowMinimum)));
}

#[test]
fn lowering_the_minimum_lets_in_a_subject_who_was_refused_before() {
    let s = setup(100);
    let bob = Address::generate(&s.env);
    give(&s, &bob, 50);
    assert_eq!(s.gate.try_enter(&bob), Err(Ok(Error::BelowMinimum)));

    s.policy
        .update(&s.policy_id, &policy_of(&s.env, &s.token, 10));
    s.gate.enter(&bob);
    assert!(s.gate.is_member(&bob));
}

#[test]
fn an_entry_that_was_recorded_stays_recorded_when_the_rule_changes() {
    // The gate records that an address was let in. It does not revoke that when the address stops qualifying: it is a record
    // of what happened, not a live entitlement. Anything that must stop applying has to ask the policy again.
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 150);
    s.gate.enter(&alice);
    s.policy
        .update(&s.policy_id, &policy_of(&s.env, &s.token, 1_000));
    assert!(s.gate.is_member(&alice));
}

// ------------------------------------------------------------------------------------------------ a balance is read when asked

#[test]
fn a_subject_let_in_is_refused_after_moving_tokens_away() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    s.gate.enter(&alice);

    TokenClient::new(&s.env, &s.token).transfer(&alice, Address::generate(&s.env), &1);
    assert_eq!(s.gate.try_enter(&alice), Err(Ok(Error::BelowMinimum)));
}

// ------------------------------------------------------------------------------------------------ fail closed

#[test]
fn a_policy_that_does_not_exist_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let gate = gate_in_front_of(&s, &s.policy.address, 999);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 1_000);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
    assert!(!gate.is_member(&alice));
}

#[test]
fn an_address_that_is_not_a_contract_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let gate = gate_in_front_of(&s, &Address::generate(&s.env), 1);
    let alice = Address::generate(&s.env);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
}

#[test]
fn a_real_contract_that_is_not_a_policy_contract_is_a_refusal_not_a_pass() {
    let s = setup(100);
    // The token exists and answers calls, but it has no `evaluate`.
    let gate = gate_in_front_of(&s, &s.token, s.policy_id);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 1_000);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
    assert!(!gate.is_member(&alice));
}

#[test]
fn a_policy_contract_that_fails_while_answering_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let broken = s.env.register(stand_ins::Panics, ());
    let gate = gate_in_front_of(&s, &broken, 1);
    let alice = Address::generate(&s.env);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
    assert!(!gate.is_member(&alice));
}

#[test]
fn an_answer_that_is_not_a_decision_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let odd = s.env.register(stand_ins::WrongType, ());
    let gate = gate_in_front_of(&s, &odd, 1);
    let alice = Address::generate(&s.env);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
    assert!(!gate.is_member(&alice));
}

#[test]
fn a_no_that_gives_no_reason_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let odd = s.env.register(stand_ins::SaysNoWithoutAReason, ());
    let gate = gate_in_front_of(&s, &odd, 1);
    let alice = Address::generate(&s.env);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
    assert!(!gate.is_member(&alice));
}

#[test]
fn the_reason_codes_declared_here_are_the_real_contracts() {
    // `DenyReason` is declared by hand. Its codes must be the ones the real contract returns, or a refusal would be misread.
    assert_eq!(DenyReason::None as u32, policy::DenyReason::None as u32);
    assert_eq!(
        DenyReason::Inactive as u32,
        policy::DenyReason::Inactive as u32
    );
    assert_eq!(
        DenyReason::BelowMinimum as u32,
        policy::DenyReason::BelowMinimum as u32
    );
    assert_eq!(
        DenyReason::BalanceUnavailable as u32,
        policy::DenyReason::BalanceUnavailable as u32
    );
    assert_eq!(
        DenyReason::BeforeWindow as u32,
        policy::DenyReason::BeforeWindow as u32
    );
    assert_eq!(
        DenyReason::AfterWindow as u32,
        policy::DenyReason::AfterWindow as u32
    );
}
