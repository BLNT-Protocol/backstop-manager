#![cfg(test)]

use soroban_sdk::{
    testutils::{
        Address as _, AuthorizedFunction, AuthorizedInvocation, EnvTestConfig, MockAuth,
        MockAuthInvoke,
    },
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, Error, IntoVal, Symbol,
};

use crate::testutils::{create_backstop_manager_wasm, create_blnt_contracts, EnvTestUtils};

#[test]
fn test_execute_comet_functions() {
    let e = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    e.cost_estimate().budget().reset_unlimited();
    e.set_default_info();
    // auths get reset and tested for each manager function
    e.mock_all_auths();

    let bombadil = Address::generate(&e);
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let (contracts, pool) = create_blnt_contracts(&e, &bombadil);
    let blnt_admin_client: &StellarAssetClient = &contracts.blnt;
    let backstop_token = &contracts.blnt_usdc_lp;

    // start manager (samwise) at scope 2
    let (_, manager_client) = create_backstop_manager_wasm(
        &e,
        &frodo,
        &samwise,
        &2,
        &vec![&e, contracts.backstop.address.clone()],
        &vec![&e, pool.clone()],
    );

    // mint tokens to the backstop manager contract
    let blnt_balance_0 = 100_000_0000000;
    let lp_balance_0 = 5000_0000000;
    blnt_admin_client.mint(&manager_client.address, &blnt_balance_0);
    backstop_token.transfer(&bombadil, &manager_client.address, &lp_balance_0);
    let blnt_token = TokenClient::new(&e, &blnt_admin_client.address);

    // validate the initial state of the backstop manager
    assert_eq!(manager_client.owner(), frodo);
    let manager = manager_client.manager();
    assert_eq!(manager.id, samwise);
    assert_eq!(manager.scope, 2);
    assert_eq!(
        manager_client.backstops(),
        vec![&e, contracts.backstop.address.clone()]
    );
    assert_eq!(manager_client.pools(), vec![&e, pool.clone()]);

    // update manager
    manager_client.set_manager(&merry, &0u32);
    assert_eq!(
        e.auths()[0],
        (
            frodo.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    manager_client.address.clone(),
                    Symbol::new(&e, "set_manager"),
                    vec![&e, merry.to_val(), 0u32.into_val(&e),]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    let manager = manager_client.manager();
    assert_eq!(manager.id, merry);
    assert_eq!(manager.scope, 0);

    // update manager - validate scope > 2 errors
    let result = manager_client.try_set_manager(&merry, &3u32);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(102))));

    // update backstops
    let new_backstops = vec![
        &e,
        Address::generate(&e),
        contracts.backstop.address.clone(),
    ];
    manager_client.set_backstops(&new_backstops);
    assert_eq!(
        e.auths()[0],
        (
            frodo.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    manager_client.address.clone(),
                    Symbol::new(&e, "set_backstops"),
                    vec![&e, new_backstops.to_val(),]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    assert_eq!(manager_client.backstops(), new_backstops);

    // update backstops - validate max contract list error
    let five_contracts = vec![
        &e,
        Address::generate(&e),
        Address::generate(&e),
        Address::generate(&e),
        Address::generate(&e),
        Address::generate(&e),
    ];
    let result = manager_client.try_set_backstops(&five_contracts);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(100))));

    // update pools
    let new_pools = vec![
        &e,
        Address::generate(&e),
        Address::generate(&e),
        pool.clone(),
        Address::generate(&e),
    ];
    manager_client.set_pools(&new_pools);
    assert_eq!(
        e.auths()[0],
        (
            frodo.clone(),
            AuthorizedInvocation {
                function: AuthorizedFunction::Contract((
                    manager_client.address.clone(),
                    Symbol::new(&e, "set_pools"),
                    vec![&e, new_pools.to_val(),]
                )),
                sub_invocations: std::vec![]
            }
        )
    );
    assert_eq!(manager_client.pools(), new_pools);

    // update pools - validate max contract list error
    let result = manager_client.try_set_pools(&five_contracts);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(100))));

    /***** TOKEN TRANSFERS ******/

    // transfer - owner can transfer tokens
    let samwise_balance_blnt_0 = blnt_token.balance(&samwise);
    let transfer_amount: i128 = 100_0000000;
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address.clone(),
                fn_name: &"transfer_token",
                args: vec![
                    &e,
                    blnt_admin_client.address.into_val(&e),
                    samwise.into_val(&e),
                    transfer_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .transfer_token(&blnt_admin_client.address, &samwise, &transfer_amount);
    assert_eq!(e.auths()[0].0, frodo); // assert require_auth exists
    let samwise_balance_blnt_1 = blnt_token.balance(&samwise);
    let balance_blnt_1 = blnt_token.balance(&manager_client.address);
    assert_eq!(
        samwise_balance_blnt_1,
        samwise_balance_blnt_0 + transfer_amount
    );
    assert_eq!(balance_blnt_1, blnt_balance_0 - transfer_amount);

    // refund - manager can refund tokens to owner at scope 0
    let frodo_balance_lp_0 = backstop_token.balance(&frodo);
    let refund_amount: i128 = 100_0000000;
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &merry,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address.clone(),
                fn_name: &"refund_token",
                args: vec![
                    &e,
                    merry.into_val(&e),
                    backstop_token.address.into_val(&e),
                    refund_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .refund_token(&merry, &backstop_token.address, &refund_amount);
    assert_eq!(e.auths()[0].0, merry); // assert require_auth exists
    let frodo_balance_lp_1 = backstop_token.balance(&frodo);
    let balance_lp_1 = backstop_token.balance(&manager_client.address);
    assert_eq!(frodo_balance_lp_1, frodo_balance_lp_0 + refund_amount);
    assert_eq!(balance_lp_1, lp_balance_0 - refund_amount);
}
