#![cfg(test)]

use soroban_sdk::{
    testutils::{
        Address as _, AuthorizedFunction, AuthorizedInvocation, EnvTestConfig, Events, MockAuth,
        MockAuthInvoke,
    },
    vec, Address, Env, Error, IntoVal, Symbol, Val, Vec,
};

use crate::testutils::{create_backstop_manager_wasm, EnvTestUtils};

/// One week in ledgers at 5 seconds per ledger (the rate `jump` uses)
const ONE_WEEK_LEDGERS: u32 = 7 * 17280;

fn setup(e: &Env) {
    e.cost_estimate().budget().reset_unlimited();
    e.set_default_info();
    e.mock_all_auths();
}

fn new_env() -> Env {
    let e = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    setup(&e);
    e
}

#[test]
fn test_ownership_transfer() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    assert_eq!(client.pending_owner(), None);

    // owner proposes merry
    let proposal_ledger = e.ledger().sequence();
    let proposal_timestamp = e.ledger().timestamp();
    client.propose_owner(&merry);
    assert_eq!(
        e.auths()[0],
        (
            frodo.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    client.address.clone(),
                    Symbol::new(&e, "propose_owner"),
                    vec![&e, merry.to_val()]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    assert_eq!(
        e.events().all().filter_by_contract(&client.address),
        vec![
            &e,
            (
                client.address.clone(),
                vec![
                    &e,
                    Symbol::new(&e, "owner_proposed").into_val(&e),
                    frodo.into_val(&e)
                ] as Vec<Val>,
                merry.into_val(&e),
            )
        ]
    );
    let pending = client.pending_owner().unwrap();
    assert_eq!(pending.id, merry);
    assert_eq!(pending.ledger, proposal_ledger);
    assert_eq!(pending.timestamp, proposal_timestamp);

    // owner is unchanged until accepted
    assert_eq!(client.owner(), frodo);

    // merry accepts in a later ledger
    e.jump(1);
    client.accept_owner();
    assert_eq!(
        e.auths()[0],
        (
            merry.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    client.address.clone(),
                    Symbol::new(&e, "accept_owner"),
                    vec![&e]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    assert_eq!(
        e.events().all().filter_by_contract(&client.address),
        vec![
            &e,
            (
                client.address.clone(),
                vec![
                    &e,
                    Symbol::new(&e, "owner_accepted").into_val(&e),
                    merry.into_val(&e)
                ] as Vec<Val>,
                frodo.into_val(&e),
            )
        ]
    );
    assert_eq!(client.owner(), merry);
    assert_eq!(client.pending_owner(), None);

    // manager is unchanged by the ownership transfer
    let manager = client.manager();
    assert_eq!(manager.id, samwise);
    assert_eq!(manager.scope, 2);

    // the old owner can no longer act as owner
    e.set_auths(&[]);
    let result = client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_manager",
                args: vec![&e, frodo.into_val(&e), 2u32.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .try_set_manager(&frodo, &2u32);
    assert!(result.is_err());

    // the new owner can
    client
        .mock_auths(&[MockAuth {
            address: &merry,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "set_manager",
                args: vec![&e, frodo.into_val(&e), 1u32.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .set_manager(&frodo, &1u32);
    assert_eq!(client.manager().id, frodo);
}

#[test]
fn test_propose_owner_requires_owner_auth() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    // the manager, even at the highest scope, cannot propose an owner
    e.set_auths(&[]);
    let result = client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "propose_owner",
                args: vec![&e, merry.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .try_propose_owner(&merry);
    assert!(result.is_err());
    assert_eq!(client.pending_owner(), None);
}

#[test]
fn test_propose_owner_invalid_owner() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    // current owner
    let result = client.try_propose_owner(&frodo);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(103))));

    // the manager contract itself
    let result = client.try_propose_owner(&client.address);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(103))));

    assert_eq!(client.pending_owner(), None);
}

#[test]
fn test_accept_owner_same_ledger() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    client.propose_owner(&merry);
    let result = client.try_accept_owner();
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(105))));
    assert_eq!(client.owner(), frodo);

    // succeeds one ledger later
    e.jump(1);
    client.accept_owner();
    assert_eq!(client.owner(), merry);
}

#[test]
fn test_accept_owner_requires_pending_owner_auth() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    client.propose_owner(&merry);
    e.jump(1);

    // neither the current owner nor the manager can accept for merry
    e.set_auths(&[]);
    for signer in [&frodo, &samwise] {
        let result = client
            .mock_auths(&[MockAuth {
                address: signer,
                invoke: &MockAuthInvoke {
                    contract: &client.address,
                    fn_name: "accept_owner",
                    args: vec![&e],
                    sub_invokes: &[],
                },
            }])
            .try_accept_owner();
        assert!(result.is_err());
    }
    assert_eq!(client.owner(), frodo);
}

#[test]
fn test_accept_owner_expiry() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    // accepting exactly one week after the proposal succeeds
    client.propose_owner(&merry);
    e.jump(ONE_WEEK_LEDGERS);
    client.accept_owner();
    assert_eq!(client.owner(), merry);

    // accepting one ledger past one week fails
    client.propose_owner(&frodo);
    e.jump(ONE_WEEK_LEDGERS + 1);
    let result = client.try_accept_owner();
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(106))));
    assert_eq!(client.owner(), merry);

    // the expired proposal is still visible until replaced or cancelled
    assert_eq!(client.pending_owner().unwrap().id, frodo);
    client.cancel_owner_proposal(&merry);
    assert_eq!(client.pending_owner(), None);
}

#[test]
fn test_propose_owner_replaces_pending() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let pippin = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    client.propose_owner(&merry);
    e.jump(ONE_WEEK_LEDGERS / 2);
    client.propose_owner(&pippin);
    let pending = client.pending_owner().unwrap();
    assert_eq!(pending.id, pippin);
    assert_eq!(pending.ledger, e.ledger().sequence());
    assert_eq!(pending.timestamp, e.ledger().timestamp());

    // merry can no longer accept
    e.jump(1);
    e.set_auths(&[]);
    let result = client
        .mock_auths(&[MockAuth {
            address: &merry,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "accept_owner",
                args: vec![&e],
                sub_invokes: &[],
            },
        }])
        .try_accept_owner();
    assert!(result.is_err());

    // the expiry restarted: pippin can accept more than a week after merry's proposal
    e.jump(ONE_WEEK_LEDGERS - 1);
    e.mock_all_auths();
    client.accept_owner();
    assert_eq!(e.auths()[0].0, pippin);
    assert_eq!(client.owner(), pippin);
}

#[test]
fn test_cancel_owner_proposal() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    // owner cancels
    client.propose_owner(&merry);
    client.cancel_owner_proposal(&frodo);
    assert_eq!(
        e.auths()[0],
        (
            frodo.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    client.address.clone(),
                    Symbol::new(&e, "cancel_owner_proposal"),
                    vec![&e, frodo.to_val()]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    assert_eq!(
        e.events().all().filter_by_contract(&client.address),
        vec![
            &e,
            (
                client.address.clone(),
                vec![
                    &e,
                    Symbol::new(&e, "owner_cancelled").into_val(&e),
                    frodo.into_val(&e)
                ] as Vec<Val>,
                merry.into_val(&e),
            )
        ]
    );
    assert_eq!(client.pending_owner(), None);
    e.jump(1);
    let result = client.try_accept_owner();
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(104))));

    // pending owner declines
    client.propose_owner(&merry);
    client.cancel_owner_proposal(&merry);
    assert_eq!(e.auths()[0].0, merry);
    assert_eq!(client.pending_owner(), None);

    // anyone else, including the manager, cannot cancel
    client.propose_owner(&merry);
    let result = client.try_cancel_owner_proposal(&samwise);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(4))));
    assert_eq!(client.pending_owner().unwrap().id, merry);

    // `from` must authorize the call
    e.set_auths(&[]);
    let result = client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "cancel_owner_proposal",
                args: vec![&e, frodo.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .try_cancel_owner_proposal(&frodo);
    assert!(result.is_err());
    assert_eq!(client.pending_owner().unwrap().id, merry);
}

#[test]
fn test_no_pending_owner() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    let result = client.try_accept_owner();
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(104))));

    let result = client.try_cancel_owner_proposal(&frodo);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(104))));
}
