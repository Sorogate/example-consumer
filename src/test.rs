extern crate std;

use soroban_sdk::{
    testutils::Address as _,
    token::{StellarAssetClient, TokenClient},
    vec, Address, Bytes, Env,
};

use crate::{Error, Gate, GateClient};

/// Sorogate's policy contract, as the network holds it: `tests/fixtures/access_policy.wasm` is the code fetched from the
/// public Testnet deployment (see the README). Importing it gives the real contract and its real types.
mod policy {
    soroban_sdk::contractimport!(file = "tests/fixtures/access_policy.wasm");
}

/// The hash of that file, as recorded in Sorogate's `docs/deployments/testnet.json`.
const DEPLOYED_WASM_SHA256: &str =
    "f702e9d267262ab3f9548fa62021b4c31bb5b5f4f3652ff713906dcd48906814";

struct Setup {
    env: Env,
    gate: GateClient<'static>,
    policy: policy::Client<'static>,
    policy_id: u64,
    token: Address,
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
    let policy_id = policy.create(
        &owner,
        &vec![
            &env,
            policy::Condition::TokenBalance(policy::TokenBalanceCond {
                token: token.clone(),
                min,
            }),
        ],
    );
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

#[test]
fn the_fixture_is_the_deployed_policy_contract() {
    let env = Env::default();
    let hash = env
        .crypto()
        .sha256(&Bytes::from_slice(&env, policy::WASM))
        .to_array();
    let hex: std::string::String = hash.iter().map(|b| std::format!("{b:02x}")).collect();
    assert_eq!(hex, DEPLOYED_WASM_SHA256);
}

#[test]
fn an_address_that_qualifies_is_let_in_and_recorded() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    assert!(!s.gate.is_member(&alice));
    s.gate.enter(&alice);
    assert!(s.gate.is_member(&alice));
}

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
fn eligibility_is_not_identity_so_qualifying_is_not_enough_without_the_subjects_signature() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    // Nobody has signed for alice. The policy would say yes; the gate must still say no.
    s.env.set_auths(&[]);
    assert!(s.gate.try_enter(&alice).is_err());
    assert!(!s.gate.is_member(&alice));
}

#[test]
fn the_owner_changing_the_rule_applies_at_once_with_no_redeploy() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 150);
    s.gate.enter(&alice);

    // The same gate, the same address: the policy owner raises the minimum.
    s.policy.update(
        &s.policy_id,
        &vec![
            &s.env,
            policy::Condition::TokenBalance(policy::TokenBalanceCond {
                token: s.token.clone(),
                min: 1_000,
            }),
        ],
    );
    let dave = Address::generate(&s.env);
    give(&s, &dave, 150);
    assert_eq!(s.gate.try_enter(&dave), Err(Ok(Error::BelowMinimum)));
}

#[test]
fn a_balance_is_read_when_asked_so_moving_tokens_away_removes_eligibility() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 100);
    TokenClient::new(&s.env, &s.token).transfer(&alice, Address::generate(&s.env), &1);
    assert_eq!(s.gate.try_enter(&alice), Err(Ok(Error::BelowMinimum)));
}

#[test]
fn a_deactivated_policy_lets_nobody_in() {
    let s = setup(100);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 1_000);
    s.policy.set_active(&s.policy_id, &false);
    assert_eq!(s.gate.try_enter(&alice), Err(Ok(Error::Inactive)));
}

#[test]
fn a_policy_that_does_not_exist_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let gate_address = s.env.register(Gate, (s.policy.address.clone(), 999_u64));
    let gate = GateClient::new(&s.env, &gate_address);
    let alice = Address::generate(&s.env);
    give(&s, &alice, 1_000);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
}

#[test]
fn an_address_that_is_not_a_policy_contract_is_a_refusal_not_a_pass() {
    let s = setup(100);
    let gate_address = s.env.register(Gate, (Address::generate(&s.env), 1_u64));
    let gate = GateClient::new(&s.env, &gate_address);
    let alice = Address::generate(&s.env);
    assert_eq!(gate.try_enter(&alice), Err(Ok(Error::PolicyUnavailable)));
}
