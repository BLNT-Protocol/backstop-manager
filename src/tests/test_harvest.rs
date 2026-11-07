#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, EnvTestConfig, Events, MockAuth, MockAuthInvoke},
    token::{StellarAssetClient, TokenClient},
    vec, Address, Env, Error, IntoVal, Symbol, Val, Vec,
};

use crate::{
    dependencies::{backstop::BackstopTier, comet::Client as CometClient},
    testutils::{
        bstop_tier, create_backstop_manager_wasm, create_blnt_contracts, mint_lp, BlntFixture,
        EnvTestUtils, ManagerTier, ONE_DAY_LEDGERS,
    },
};

/// LP tokens sent to the manager contract, and deposited, by `Harness::new`
const LP_BALANCE: i128 = 1_000_0000000;
const DEPOSIT: i128 = 100_0000000;

fn new_env() -> Env {
    let e = Env::new_with_config(EnvTestConfig {
        capture_snapshot_at_drop: false,
    });
    e.cost_estimate().budget().reset_unlimited();
    e.set_default_info();
    e.mock_all_auths();
    e
}

/// A BLNT v3 environment with a manager contract that holds a backstop position
/// of DEPOSIT LP tokens in `tier`, with `samwise` as a High scope manager.
struct Harness<'a> {
    e: Env,
    bombadil: Address,
    frodo: Address,
    samwise: Address,
    contracts: BlntFixture<'a>,
    pool: Address,
    tier: ManagerTier,
    b_tier: BackstopTier,
    client: crate::testutils::ManagerClient<'a>,
}

impl<'a> Harness<'a> {
    fn new(tier: ManagerTier) -> Self {
        let e = new_env();
        let bombadil = Address::generate(&e);
        let frodo = Address::generate(&e);
        let samwise = Address::generate(&e);
        let (contracts, pool) = create_blnt_contracts(&e, &bombadil);
        let b_tier = bstop_tier(&tier);
        let (_, client) = create_backstop_manager_wasm(
            &e,
            &frodo,
            &samwise,
            &2,
            &vec![&e, contracts.backstop.address.clone()],
            &vec![&e, pool.clone()],
        );
        contracts
            .tier_lp(&b_tier)
            .transfer(&bombadil, &client.address, &LP_BALANCE);
        let h = Harness {
            e,
            bombadil,
            frodo,
            samwise,
            contracts,
            pool,
            tier,
            b_tier,
            client,
        };
        h.client.b_deposit(
            &h.frodo,
            &h.contracts.backstop.address,
            &h.tier,
            &h.pool,
            &DEPOSIT,
        );
        h
    }

    fn lp(&self) -> &CometClient<'_> {
        self.contracts.tier_lp(&self.b_tier)
    }

    fn principal(&self) -> i128 {
        self.client
            .principal(&self.contracts.backstop.address, &self.pool, &self.tier)
    }

    fn harvestable(&self) -> i128 {
        self.client.harvestable(&self.lp().address)
    }

    fn shares(&self) -> i128 {
        self.contracts
            .backstop
            .user_balance(&self.b_tier, &self.pool, &self.client.address)
            .shares
    }

    /// (tokens, shares) of the tier
    fn tier_balance(&self) -> (i128, i128) {
        let index = match self.b_tier {
            BackstopTier::FirstLoss => 0,
            BackstopTier::SecondLoss => 1,
            BackstopTier::ThirdLoss => 2,
        };
        let data = self
            .contracts
            .backstop
            .pool_data(&self.pool)
            .tiers
            .get(index)
            .unwrap();
        (data.tokens, data.shares)
    }

    /// The value of the manager's position in LP tokens, from the backstop's pool data
    fn position_value(&self) -> i128 {
        let (tokens, shares) = self.tier_balance();
        self.shares() * tokens / shares
    }

    /// Simulate interest by donating `amount` LP tokens to the tier
    fn donate(&self, amount: i128) {
        mint_lp(&self.e, self.lp(), &self.bombadil, amount);
        self.lp().approve(
            &self.bombadil,
            &self.contracts.backstop.address,
            &amount,
            &(self.e.ledger().sequence() + 100),
        );
        self.contracts
            .backstop
            .donate(&self.b_tier, &self.bombadil, &self.pool, &amount);
    }

    /// Simulate a loss by drawing `amount` LP tokens from the tier
    fn draw(&self, amount: i128) {
        self.contracts
            .backstop
            .draw(&self.b_tier, &self.pool, &amount, &self.bombadil);
    }

    /// Queue `shares` as the manager, wait out the queue, and withdraw them.
    /// Returns the LP tokens returned.
    fn withdraw(&self, shares: i128) -> i128 {
        self.client.b_queue_withdrawal(
            &self.samwise,
            &self.contracts.backstop.address,
            &self.tier,
            &self.pool,
            &shares,
        );
        self.e.jump(17 * ONE_DAY_LEDGERS + 1);
        self.client.b_withdraw(
            &self.samwise,
            &self.contracts.backstop.address,
            &self.tier,
            &self.pool,
            &shares,
        )
    }

    /// The `earnings_realized` event emitted by the last call
    fn assert_earnings_realized(
        &self,
        principal_part: i128,
        earnings_part: i128,
        manager_part: i128,
    ) {
        let e = &self.e;
        assert_eq!(
            e.events().all().filter_by_contract(&self.client.address),
            vec![
                e,
                (
                    self.client.address.clone(),
                    vec![
                        e,
                        Symbol::new(e, "earnings_realized").into_val(e),
                        self.contracts.backstop.address.into_val(e),
                        self.pool.into_val(e),
                    ] as Vec<Val>,
                    (vec![
                        e,
                        self.tier.into_val(e),
                        principal_part.into_val(e),
                        earnings_part.into_val(e),
                        manager_part.into_val(e),
                    ] as Vec<Val>)
                        .into_val(e),
                )
            ]
        );
    }
}

#[test]
fn test_harvest_earnings_first_first_loss() {
    run_harvest_earnings_first(ManagerTier::FirstLoss);
}

#[test]
fn test_harvest_earnings_first_second_loss() {
    run_harvest_earnings_first(ManagerTier::SecondLoss);
}

fn run_harvest_earnings_first(tier: ManagerTier) {
    let h = Harness::new(tier);
    let lp_token = TokenClient::new(&h.e, &h.lp().address);
    h.client.set_harvest_bps(&10_000);

    // deposits add to principal
    assert_eq!(h.principal(), DEPOSIT);
    assert_eq!(h.harvestable(), 0);

    // interest doubles the value of the tier's shares; the share count is unchanged
    let shares_0 = h.shares();
    let (tier_tokens, _) = h.tier_balance();
    h.donate(tier_tokens);
    assert_eq!(h.shares(), shares_0);
    let value_0 = h.position_value();
    assert!(value_0 > DEPOSIT * 19 / 10);

    // withdrawing a quarter of the shares returns less than the earnings,
    // so it is all earnings and principal is untouched
    let q1 = shares_0 / 4;
    let returned_1 = h.withdraw(q1);
    assert!(returned_1 < value_0 - DEPOSIT);
    h.assert_earnings_realized(0, returned_1, returned_1);
    assert_eq!(h.principal(), DEPOSIT);
    assert_eq!(h.harvestable(), returned_1);

    // withdrawing half of the remaining shares returns more than the remaining
    // earnings, so the rest comes out of principal
    let shares_1 = shares_0 - q1;
    let value_1 = h.position_value();
    let q2 = shares_1 / 2;
    let returned_2 = h.withdraw(q2);
    let earnings_2 = value_1 - DEPOSIT;
    assert!(returned_2 > earnings_2 && earnings_2 > 0);
    h.assert_earnings_realized(returned_2 - earnings_2, earnings_2, earnings_2);
    assert_eq!(h.principal(), DEPOSIT - (returned_2 - earnings_2));
    assert_eq!(h.harvestable(), returned_1 + earnings_2);

    // the manager harvests its earnings
    let samwise_lp_0 = lp_token.balance(&h.samwise);
    let contract_lp_0 = lp_token.balance(&h.client.address);
    let harvestable = h.harvestable();
    let paid = h.client.harvest(&h.samwise, &h.lp().address);
    assert_eq!(paid, harvestable);
    assert_eq!(
        h.e.events().all().filter_by_contract(&h.client.address),
        vec![
            &h.e,
            (
                h.client.address.clone(),
                vec![
                    &h.e,
                    Symbol::new(&h.e, "harvest").into_val(&h.e),
                    h.lp().address.into_val(&h.e),
                    h.samwise.into_val(&h.e),
                ] as Vec<Val>,
                paid.into_val(&h.e),
            )
        ]
    );
    assert_eq!(lp_token.balance(&h.samwise), samwise_lp_0 + paid);
    assert_eq!(lp_token.balance(&h.client.address), contract_lp_0 - paid);
    assert_eq!(h.harvestable(), 0);

    // claiming emissions adds shares but not principal
    let principal = h.principal();
    let shares_2 = h.shares();
    h.e.jump(ONE_DAY_LEDGERS);
    h.client.b_claim(
        &h.samwise,
        &h.contracts.backstop.address,
        &h.tier,
        &h.pool,
        &0,
    );
    assert!(h.shares() > shares_2);
    assert_eq!(h.principal(), principal);

    // the manager's own principal tokens can't be harvested
    assert_eq!(h.client.harvest(&h.samwise, &h.lp().address), 0);
}

#[test]
fn test_small_rounded_withdrawal_uses_exact_position_value() {
    let h = Harness::new(ManagerTier::SecondLoss);
    h.client.set_harvest_bps(&10_000);

    // Establish a non-integral token-per-share rate. Withdrawing one share
    // returns one token base unit after rounding, but the full position is
    // worth substantially more than its principal.
    let (tier_tokens, _) = h.tier_balance();
    h.donate(tier_tokens / 2);
    assert!(h.position_value() > h.principal());

    let principal = h.principal();
    let returned = h.withdraw(1);
    assert_eq!(returned, 1);
    h.assert_earnings_realized(0, returned, returned);
    assert_eq!(h.principal(), principal);
    assert_eq!(h.harvestable(), returned);
}

#[test]
fn test_harvest_bps_share() {
    let h = Harness::new(ManagerTier::SecondLoss);
    h.client.set_harvest_bps(&2_500);

    let (tier_tokens, _) = h.tier_balance();
    h.donate(tier_tokens);

    // all earnings; the manager gets 25% and the owner's 75% stays in the contract
    let returned = h.withdraw(h.shares() / 4);
    let manager_part = returned * 2_500 / 10_000;
    h.assert_earnings_realized(0, returned, manager_part);
    assert_eq!(h.harvestable(), manager_part);
    assert_eq!(h.principal(), DEPOSIT);

    // with harvest_bps at 0, earnings are realized but nothing is owed
    h.client.set_harvest_bps(&0);
    let returned = h.withdraw(h.shares() / 4);
    h.assert_earnings_realized(0, returned, 0);
    assert_eq!(h.harvestable(), manager_part);
}

#[test]
fn test_redeposit_harvestable_turns_it_into_principal() {
    let h = Harness::new(ManagerTier::SecondLoss);
    let lp_token = TokenClient::new(&h.e, &h.lp().address);
    h.client.set_harvest_bps(&10_000);

    // Leave some non-harvestable LP tokens liquid so deposits can prove they
    // consume owner liquidity before consuming the manager's earnings.
    let liquid_balance = lp_token.balance(&h.client.address);
    h.client
        .transfer_token(&h.lp().address, &h.frodo, &(liquid_balance - DEPOSIT));

    let (tier_tokens, _) = h.tier_balance();
    h.donate(tier_tokens);
    let returned = h.withdraw(h.shares() / 4);
    assert_eq!(h.harvestable(), returned);
    assert_eq!(lp_token.balance(&h.client.address), DEPOSIT + returned);

    let principal = h.principal();
    h.client.b_deposit(
        &h.samwise,
        &h.contracts.backstop.address,
        &h.tier,
        &h.pool,
        &DEPOSIT,
    );
    assert_eq!(h.principal(), principal + DEPOSIT);
    assert_eq!(h.harvestable(), returned);

    h.client.b_deposit(
        &h.samwise,
        &h.contracts.backstop.address,
        &h.tier,
        &h.pool,
        &returned,
    );

    assert_eq!(h.principal(), principal + DEPOSIT + returned);
    assert_eq!(h.harvestable(), 0);
    assert_eq!(h.client.harvest(&h.samwise, &h.lp().address), 0);
}

#[test]
fn test_harvest_after_loss() {
    let h = Harness::new(ManagerTier::FirstLoss);
    h.client.set_harvest_bps(&10_000);

    // a draw halves the value of the tier's shares
    let (tier_tokens, _) = h.tier_balance();
    h.draw(tier_tokens / 2);
    assert!(h.position_value() < DEPOSIT);

    // with value below principal, a withdrawal is all principal
    let shares_0 = h.shares();
    let q1 = shares_0 / 4;
    let returned_1 = h.withdraw(q1);
    h.assert_earnings_realized(returned_1, 0, 0);
    assert_eq!(h.principal(), DEPOSIT - returned_1);
    assert_eq!(h.harvestable(), 0);

    // interest recovers the value above principal; only the value above
    // principal is earnings
    let (tier_tokens, _) = h.tier_balance();
    h.donate(tier_tokens * 2);
    let principal = h.principal();
    let shares_1 = h.shares();
    let value_1 = h.position_value();
    assert!(value_1 > principal);
    let q2 = shares_1 / 2;
    let returned_2 = h.withdraw(q2);
    let earnings = value_1 - principal;
    assert!(returned_2 > earnings && earnings > 0);
    h.assert_earnings_realized(returned_2 - earnings, earnings, earnings);
    assert_eq!(h.principal(), principal - (returned_2 - earnings));
    assert_eq!(h.harvestable(), earnings);
}

#[test]
fn test_set_principal_for_existing_position() {
    let h = Harness::new(ManagerTier::SecondLoss);
    h.client.set_harvest_bps(&10_000);

    // the owner can set principal directly, e.g. for a position the contract
    // held before principal was tracked
    h.client
        .set_principal(&h.contracts.backstop.address, &h.pool, &h.tier, &0);
    assert_eq!(
        h.e.events().all().filter_by_contract(&h.client.address),
        vec![
            &h.e,
            (
                h.client.address.clone(),
                vec![
                    &h.e,
                    Symbol::new(&h.e, "principal_set").into_val(&h.e),
                    h.contracts.backstop.address.into_val(&h.e),
                    h.pool.into_val(&h.e),
                ] as Vec<Val>,
                (vec![&h.e, h.tier.into_val(&h.e), 0i128.into_val(&h.e)] as Vec<Val>)
                    .into_val(&h.e),
            )
        ]
    );
    assert_eq!(h.principal(), 0);

    // with no principal, the whole position is earnings
    let returned = h.withdraw(h.shares() / 2);
    h.assert_earnings_realized(0, returned, returned);
    assert_eq!(h.harvestable(), returned);
    assert_eq!(h.principal(), 0);
}

#[test]
fn test_harvest() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let merry = Address::generate(&e);
    let pippin = Address::generate(&e);
    let token = StellarAssetClient::new(
        &e,
        &e.register_stellar_asset_contract_v2(frodo.clone())
            .address(),
    );
    let token_client = TokenClient::new(&e, &token.address);

    // samwise is a Low scope manager
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &0, &vec![&e], &vec![&e]);

    // nothing harvestable
    assert_eq!(client.harvest(&samwise, &token.address), 0);

    // pays no more than the contract holds
    client.set_harvestable(&token.address, &100);
    assert_eq!(
        e.events().all().filter_by_contract(&client.address),
        vec![
            &e,
            (
                client.address.clone(),
                vec![
                    &e,
                    Symbol::new(&e, "harvestable_set").into_val(&e),
                    token.address.into_val(&e),
                ] as Vec<Val>,
                100i128.into_val(&e),
            )
        ]
    );
    token.mint(&client.address, &60);
    assert_eq!(client.harvest(&samwise, &token.address), 60);
    assert_eq!(e.auths()[0].0, samwise);
    assert_eq!(token_client.balance(&samwise), 60);
    assert_eq!(client.harvestable(&token.address), 40);

    // nothing to pay with an empty balance
    assert_eq!(client.harvest(&samwise, &token.address), 0);
    assert_eq!(client.harvestable(&token.address), 40);

    // the owner can call harvest, but the tokens go to the manager
    token.mint(&client.address, &100);
    assert_eq!(client.harvest(&frodo, &token.address), 40);
    assert_eq!(token_client.balance(&samwise), 100);
    assert_eq!(token_client.balance(&frodo), 0);
    assert_eq!(client.harvestable(&token.address), 0);
    assert_eq!(token_client.balance(&client.address), 60);

    // no one else can harvest
    client.set_harvestable(&token.address, &10);
    let result = client.try_harvest(&pippin, &token.address);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(4))));

    // `from` must authorize the call
    e.set_auths(&[]);
    let result = client
        .mock_auths(&[MockAuth {
            address: &pippin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "harvest",
                args: vec![&e, samwise.into_val(&e), token.address.into_val(&e)],
                sub_invokes: &[],
            },
        }])
        .try_harvest(&samwise, &token.address);
    assert!(result.is_err());
    e.mock_all_auths();

    // harvestable balances belong to the manager role: after a change,
    // the new manager is paid
    client.set_manager(&merry, &0);
    assert_eq!(
        client.try_harvest(&samwise, &token.address).err(),
        Some(Ok(Error::from_contract_error(4)))
    );
    assert_eq!(client.harvest(&merry, &token.address), 10);
    assert_eq!(token_client.balance(&merry), 10);
    assert_eq!(token_client.balance(&samwise), 100);
}

#[test]
fn test_owner_harvest_settings() {
    let e = new_env();
    let frodo = Address::generate(&e);
    let samwise = Address::generate(&e);
    let backstop = Address::generate(&e);
    let pool = Address::generate(&e);
    let token = Address::generate(&e);
    let (_, client) = create_backstop_manager_wasm(&e, &frodo, &samwise, &2, &vec![&e], &vec![&e]);

    // defaults
    assert_eq!(client.harvest_bps(), 0);
    assert_eq!(
        client.principal(&backstop, &pool, &ManagerTier::FirstLoss),
        0
    );
    assert_eq!(client.harvestable(&token), 0);

    // harvest_bps
    client.set_harvest_bps(&10_000);
    assert_eq!(e.auths()[0].0, frodo);
    assert_eq!(
        e.events().all().filter_by_contract(&client.address),
        vec![
            &e,
            (
                client.address.clone(),
                vec![&e, Symbol::new(&e, "harvest_bps_set").into_val(&e)] as Vec<Val>,
                10_000u32.into_val(&e),
            )
        ]
    );
    assert_eq!(client.harvest_bps(), 10_000);
    let result = client.try_set_harvest_bps(&10_001);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(107))));
    assert_eq!(client.harvest_bps(), 10_000);

    // principal is per position
    client.set_principal(&backstop, &pool, &ManagerTier::FirstLoss, &50);
    assert_eq!(e.auths()[0].0, frodo);
    assert_eq!(
        client.principal(&backstop, &pool, &ManagerTier::FirstLoss),
        50
    );
    assert_eq!(
        client.principal(&backstop, &pool, &ManagerTier::SecondLoss),
        0
    );
    let result = client.try_set_principal(&backstop, &pool, &ManagerTier::FirstLoss, &-1);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(8))));

    // harvestable
    client.set_harvestable(&token, &25);
    assert_eq!(e.auths()[0].0, frodo);
    assert_eq!(client.harvestable(&token), 25);
    let result = client.try_set_harvestable(&token, &-1);
    assert_eq!(result.err(), Some(Ok(Error::from_contract_error(8))));

    // the manager, even at High scope, can't change any of them
    e.set_auths(&[]);
    let invoke = |fn_name: &'static str, args: Vec<Val>| MockAuthInvoke {
        contract: &client.address,
        fn_name,
        args,
        sub_invokes: &[],
    };
    let set_bps = invoke("set_harvest_bps", vec![&e, 0u32.into_val(&e)]);
    assert!(client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &set_bps,
        }])
        .try_set_harvest_bps(&0)
        .is_err());
    let set_principal = invoke(
        "set_principal",
        vec![
            &e,
            backstop.into_val(&e),
            pool.into_val(&e),
            ManagerTier::FirstLoss.into_val(&e),
            0i128.into_val(&e),
        ],
    );
    assert!(client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &set_principal,
        }])
        .try_set_principal(&backstop, &pool, &ManagerTier::FirstLoss, &0)
        .is_err());
    let set_harvestable = invoke(
        "set_harvestable",
        vec![&e, token.into_val(&e), 1_000i128.into_val(&e)],
    );
    assert!(client
        .mock_auths(&[MockAuth {
            address: &samwise,
            invoke: &set_harvestable,
        }])
        .try_set_harvestable(&token, &1_000)
        .is_err());
    assert_eq!(client.harvest_bps(), 10_000);
    assert_eq!(
        client.principal(&backstop, &pool, &ManagerTier::FirstLoss),
        50
    );
    assert_eq!(client.harvestable(&token), 25);
}
