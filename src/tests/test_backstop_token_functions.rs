#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, Error, IntoVal, Vec,
};

use crate::testutils::{
    bstop_tier, create_backstop_manager_wasm, create_blnt_contracts, EnvTestUtils, ManagerTier,
};

#[test]
fn test_execute_comet_functions_first_loss() {
    run_comet_functions(ManagerTier::FirstLoss);
}

#[test]
fn test_execute_comet_functions_second_loss() {
    run_comet_functions(ManagerTier::SecondLoss);
}

fn run_comet_functions(tier: ManagerTier) {
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
    let random_address = Address::generate(&e);
    let (contracts, pool) = create_blnt_contracts(&e, &bombadil);
    let b_tier = bstop_tier(&tier);
    let backstop_token = contracts.tier_lp(&b_tier);
    // FirstLoss is BLNT:XLM, SecondLoss is BLNT:USDC
    let blnt_admin_client: &StellarAssetClient = &contracts.blnt;
    let pair_admin_client: &StellarAssetClient = match tier {
        ManagerTier::FirstLoss => &contracts.xlm,
        _ => &contracts.usdc,
    };

    // start manager (samwise) at scope 1
    let (_, manager_client) = create_backstop_manager_wasm(
        &e,
        &frodo,
        &samwise,
        &1,
        &vec![&e, contracts.backstop.address.clone()],
        &vec![&e, pool.clone()],
    );

    // mint underlying tokens to the backstop manager contract
    // create_blnt_contracts sets up each comet LP with
    //  -> 10 BLNT / share
    //  -> 0.25 pair token / share
    let blnt_balance_0 = 100_000_0000000;
    let pair_balance_0 = 2500_0000000;
    blnt_admin_client.mint(&manager_client.address, &blnt_balance_0);
    pair_admin_client.mint(&manager_client.address, &pair_balance_0);
    let blnt_token = TokenClient::new(&e, &blnt_admin_client.address);
    let pair_token = TokenClient::new(&e, &pair_admin_client.address);

    /***** SCOPE 1 *****/

    // join pool - as owner
    let lp_mint_amount = 100_0000000;
    let max_amount_in: Vec<i128> = vec![&e, 1001_0000000, 25_1000000];
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_join_pool",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    max_amount_in.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .c_join_pool(
            &frodo,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &max_amount_in,
        );
    assert_eq!(e.auths()[0].0, frodo); // assert require_auth exists
    let blnt_balance_1 = blnt_token.balance(&manager_client.address);
    let pair_balance_1 = pair_token.balance(&manager_client.address);
    let lp_balance_1 = backstop_token.balance(&manager_client.address);
    assert!(blnt_balance_1 >= blnt_balance_0 - max_amount_in.get_unchecked(0));
    assert!(pair_balance_1 >= pair_balance_0 - max_amount_in.get_unchecked(1));
    assert_eq!(lp_balance_1, lp_mint_amount);

    // join pool - validate requires scope 2 as manager
    e.set_auths(&[]);
    let join_scope_1 = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_join_pool",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    max_amount_in.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_c_join_pool(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &max_amount_in,
        );
    assert_eq!(join_scope_1.err(), Some(Ok(Error::from_contract_error(4))));

    // exit pool - validate requires scope 2 as manager
    let min_amount_out: Vec<i128> = vec![&e, 999_0000000, 24_9000000];
    let exit_scope_1 = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_exit_pool",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    min_amount_out.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_c_exit_pool(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &min_amount_out,
        );
    assert_eq!(exit_scope_1.err(), Some(Ok(Error::from_contract_error(4))));

    // exit pool - as owner
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_exit_pool",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    min_amount_out.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .c_exit_pool(
            &frodo,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &min_amount_out,
        );
    assert_eq!(e.auths()[0].0, frodo); // assert require_auth exists
    let blnt_balance_2 = blnt_token.balance(&manager_client.address);
    let pair_balance_2 = pair_token.balance(&manager_client.address);
    let lp_balance_2 = backstop_token.balance(&manager_client.address);
    assert!(blnt_balance_2 >= blnt_balance_1 + min_amount_out.get_unchecked(0));
    assert!(pair_balance_2 >= pair_balance_1 + min_amount_out.get_unchecked(1));
    assert_eq!(lp_balance_2, 0);

    // join / exit pool - validates contracts
    e.set_auths(&[]);
    let join_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_join_pool",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    random_address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    max_amount_in.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_c_join_pool(
            &frodo,
            &random_address,
            &tier,
            &pool,
            &lp_mint_amount,
            &max_amount_in,
        );
    assert_eq!(
        join_invalid.err(),
        Some(Ok(Error::from_contract_error(101)))
    );
    let exit_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_exit_pool",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    random_address.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    min_amount_out.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_c_exit_pool(
            &frodo,
            &contracts.backstop.address,
            &tier,
            &random_address,
            &lp_mint_amount,
            &min_amount_out,
        );
    assert_eq!(
        exit_invalid.err(),
        Some(Ok(Error::from_contract_error(101)))
    );

    /***** SCOPE 2 *****/

    // set manager (samwise) at scope 2
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"set_manager",
                args: vec![&e, samwise.into_val(&e), 2u32.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .set_manager(&samwise, &2u32);

    let manager = manager_client.manager();
    assert_eq!(manager.id, samwise);
    assert_eq!(manager.scope, 2);

    // join pool - as manager
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_join_pool",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    max_amount_in.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .c_join_pool(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &max_amount_in,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let blnt_balance_3 = blnt_token.balance(&manager_client.address);
    let pair_balance_3 = pair_token.balance(&manager_client.address);
    let lp_balance_3 = backstop_token.balance(&manager_client.address);
    assert!(blnt_balance_3 >= blnt_balance_2 - max_amount_in.get_unchecked(0));
    assert!(pair_balance_3 >= pair_balance_2 - max_amount_in.get_unchecked(1));
    assert_eq!(lp_balance_3, lp_mint_amount);

    // exit pool - as manager
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"c_exit_pool",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    lp_mint_amount.into_val(&e),
                    min_amount_out.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .c_exit_pool(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &lp_mint_amount,
            &min_amount_out,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let blnt_balance_4 = blnt_token.balance(&manager_client.address);
    let pair_balance_4 = pair_token.balance(&manager_client.address);
    let lp_balance_4 = backstop_token.balance(&manager_client.address);
    assert!(blnt_balance_4 >= blnt_balance_3 + min_amount_out.get_unchecked(0));
    assert!(pair_balance_4 >= pair_balance_3 + min_amount_out.get_unchecked(1));
    assert_eq!(lp_balance_4, 0);
}
