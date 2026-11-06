#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, MockAuth, MockAuthInvoke},
    vec, Address, Env, Error, IntoVal,
};

use crate::testutils::{
    bstop_tier, create_backstop_manager_wasm, create_blnt_contracts, EnvTestUtils, ManagerTier,
    ONE_DAY_LEDGERS,
};

#[test]
fn test_execute_backstop_functions_first_loss() {
    run_backstop_functions(ManagerTier::FirstLoss);
}

#[test]
fn test_execute_backstop_functions_second_loss() {
    run_backstop_functions(ManagerTier::SecondLoss);
}

fn run_backstop_functions(tier: ManagerTier) {
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
    let (contracts, pool) = create_blnt_contracts(&e, &bombadil);
    let b_tier = bstop_tier(&tier);
    let backstop_token = contracts.tier_lp(&b_tier);

    let random_address = Address::generate(&e);

    // start manager (samwise) at scope 0
    let (_, manager_client) = create_backstop_manager_wasm(
        &e,
        &frodo,
        &samwise,
        &0,
        &vec![&e, contracts.backstop.address.clone()],
        &vec![&e, pool.clone()],
    );

    // bombadil sent LP tokens to the backstop manager contract
    let lp_mint_amount = 1_000_0000000;
    backstop_token.transfer(&bombadil, &manager_client.address, &lp_mint_amount);

    let deposit_amount = lp_mint_amount / 2;

    /***** SCOPE 0 *****/

    // assert scope is 0
    let manager = manager_client.manager();
    assert_eq!(0, manager.scope);
    assert_eq!(samwise, manager.id);

    // deposit - validates contracts
    let deposit_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_deposit",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    random_address.into_val(&e),
                    deposit_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_deposit(
            &frodo,
            &contracts.backstop.address,
            &tier,
            &random_address,
            &deposit_amount,
        );
    assert_eq!(
        deposit_invalid.err(),
        Some(Ok(Error::from_contract_error(101)))
    );

    // deposit - as owner
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_deposit",
                args: vec![
                    &e,
                    frodo.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    deposit_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_deposit(
            &frodo,
            &contracts.backstop.address,
            &tier,
            &pool,
            &deposit_amount,
        );
    assert_eq!(e.auths()[0].0, frodo); // assert require_auth exists
    let backstop_bal_0 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert_eq!(deposit_amount, backstop_bal_0.shares);

    // - allow some time to pass to accrue emissions
    e.jump(ONE_DAY_LEDGERS);

    // claim - as manager
    let min_lp_amount = 0_1000000;
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_claim",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    min_lp_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_claim(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &min_lp_amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_1 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert!(backstop_bal_1.shares > backstop_bal_0.shares);

    // deposit - validate scope 0 cannot invoke scope 1 functions
    e.set_auths(&[]);
    let scope_1_req_result = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_deposit",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    deposit_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_deposit(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &deposit_amount,
        );
    assert_eq!(
        scope_1_req_result.err(),
        Some(Ok(Error::from_contract_error(4)))
    );

    /***** SCOPE 1 ****/

    // set manager to scope 1
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &frodo,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"set_manager",
                args: vec![&e, samwise.into_val(&e), 1u32.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .set_manager(&samwise, &1u32);

    // assert scope is 1
    let manager = manager_client.manager();
    assert_eq!(1, manager.scope);
    assert_eq!(samwise, manager.id);

    // deposit - as the manager
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_deposit",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    deposit_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_deposit(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &deposit_amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_2 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert_eq!(
        deposit_amount + backstop_bal_1.shares,
        backstop_bal_2.shares
    );

    // q4w - validates contracts
    let q4w_amount = deposit_amount; // ~ half of total shares
    let q4w_start_time = e.ledger().timestamp();
    e.set_auths(&[]);
    let q4w_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_queue_withdrawal",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    random_address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    q4w_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_queue_withdrawal(&samwise, &random_address, &tier, &pool, &q4w_amount);
    assert_eq!(q4w_invalid.err(), Some(Ok(Error::from_contract_error(101))));

    // q4w - as the manager
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_queue_withdrawal",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    q4w_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_queue_withdrawal(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &q4w_amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_3 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert_eq!(backstop_bal_2.shares - q4w_amount, backstop_bal_3.shares);
    assert_eq!(backstop_bal_3.q4w.len(), 1);
    let q4w = backstop_bal_3.q4w.get_unchecked(0);
    assert_eq!(q4w_amount, q4w.amount);
    assert_eq!(q4w_start_time + 17 * 24 * 60 * 60, q4w.exp);

    // - wait a day
    e.jump(ONE_DAY_LEDGERS);

    // dequeue q4w - validates contracts
    let dequeue_amount = q4w_amount / 2;
    e.set_auths(&[]);
    let dequeue_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_dequeue_withdrawal",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    random_address.into_val(&e),
                    dequeue_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_dequeue_withdrawal(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &random_address,
            &dequeue_amount,
        );
    assert_eq!(
        dequeue_invalid.err(),
        Some(Ok(Error::from_contract_error(101)))
    );

    // dequeue q4w - as the manager
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_dequeue_withdrawal",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    dequeue_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_dequeue_withdrawal(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &dequeue_amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_4 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert_eq!(
        backstop_bal_2.shares - dequeue_amount,
        backstop_bal_4.shares
    );
    assert_eq!(backstop_bal_4.q4w.len(), 1);
    let q4w = backstop_bal_4.q4w.get_unchecked(0);
    assert_eq!(q4w_amount - dequeue_amount, q4w.amount);
    assert_eq!(q4w_start_time + 17 * 24 * 60 * 60, q4w.exp);

    // - wait until q4w expires
    e.jump(16 * ONE_DAY_LEDGERS + 1);

    // withdraw - validate scope 1 cannot invoke scope 2 functions
    e.set_auths(&[]);
    let withdraw_scope_1 = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_withdraw",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    q4w.amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_withdraw(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &q4w.amount,
        );
    assert_eq!(
        withdraw_scope_1.err(),
        Some(Ok(Error::from_contract_error(4)))
    );

    /***** SCOPE 2 *****/

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

    // assert scope is 2
    let manager = manager_client.manager();
    assert_eq!(2, manager.scope);
    assert_eq!(samwise, manager.id);

    // withdraw - validates contracts
    e.set_auths(&[]);
    let withdraw_invalid = manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_withdraw",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    random_address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    q4w.amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .try_b_withdraw(&samwise, &random_address, &tier, &pool, &q4w.amount);
    assert_eq!(
        withdraw_invalid.err(),
        Some(Ok(Error::from_contract_error(101)))
    );

    // withdraw - as the manager
    let backstop_token_bal_0 = backstop_token.balance(&manager_client.address);
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_withdraw",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    q4w.amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_withdraw(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &q4w.amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_5 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert_eq!(backstop_bal_4.shares, backstop_bal_5.shares);
    assert_eq!(0, backstop_bal_5.q4w.len());
    assert_eq!(
        backstop_token_bal_0 + q4w.amount,
        backstop_token.balance(&manager_client.address)
    );

    // - allow some time to pass to accrue emissions
    e.jump(ONE_DAY_LEDGERS);

    // claim - validate manager can invoke lower scope functions
    e.set_auths(&[]);
    manager_client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &MockAuthInvoke {
                contract: &manager_client.address,
                fn_name: &"b_claim",
                args: vec![
                    &e,
                    samwise.into_val(&e),
                    contracts.backstop.address.into_val(&e),
                    tier.into_val(&e),
                    pool.into_val(&e),
                    min_lp_amount.into_val(&e),
                ],
                sub_invokes: &[],
            },
        }])
        .b_claim(
            &samwise,
            &contracts.backstop.address,
            &tier,
            &pool,
            &min_lp_amount,
        );
    assert_eq!(e.auths()[0].0, samwise); // assert require_auth exists
    let backstop_bal_6 = contracts
        .backstop
        .user_balance(&b_tier, &pool, &manager_client.address);
    assert!(backstop_bal_6.shares > backstop_bal_5.shares);
}
