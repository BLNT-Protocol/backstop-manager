#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, BytesN as _, Ledger as _, LedgerInfo},
    token::StellarAssetClient,
    vec, Address, BytesN, Env, String, Vec,
};

use crate::dependencies::{
    backstop::{BackstopTier, Client as BackstopClient, WASM as BACKSTOP_WASM},
    comet::{Client as CometClient, WASM as COMET_WASM},
};

mod contract {
    soroban_sdk::contractimport!(file = "./target/wasm32v1-none/optimized/backstop_manager.wasm");
}

/// BLNT v3 contracts used only to build a test environment. Built from
/// `blnt-contracts-v3` (see `src/dependencies/README.md`).
pub mod v3 {
    pub mod emitter {
        soroban_sdk::contractimport!(file = "./src/dependencies/test/emitter.wasm");
    }
    pub mod pool_factory {
        soroban_sdk::contractimport!(file = "./src/dependencies/test/pool_factory.wasm");
    }
    pub mod pool {
        soroban_sdk::contractimport!(file = "./src/dependencies/test/pool.wasm");
    }
}

/// The manager's `BackstopTier`, as used by the manager client
pub use contract::BackstopTier as ManagerTier;

/// Client for the backstop manager wasm
pub use contract::Client as ManagerClient;

/// Convert a manager tier into the backstop's tier type
pub fn bstop_tier(tier: &ManagerTier) -> BackstopTier {
    match tier {
        ManagerTier::FirstLoss => BackstopTier::FirstLoss,
        ManagerTier::SecondLoss => BackstopTier::SecondLoss,
        ManagerTier::ThirdLoss => BackstopTier::ThirdLoss,
    }
}

/// Create a backstop manager contract via wasm
///
/// ### Arguments
/// * owner - The address of the owner of the funds
/// * manager - The address of the manager of the funds
/// * admin_scope - The scope of the manager
/// * backstops - The addresses of the backstops the manager can interact with initially
/// * pools - The addresses of the pools the manager can interact with initially
pub fn create_backstop_manager_wasm<'a>(
    e: &Env,
    owner: &Address,
    manager: &Address,
    admin_scope: &u32,
    backstops: &Vec<Address>,
    pools: &Vec<Address>,
) -> (Address, contract::Client<'a>) {
    let backstop_manager_address = e.register(
        contract::WASM,
        (
            owner,
            manager,
            admin_scope,
            backstops.clone(),
            pools.clone(),
        ),
    );
    let backstop_manager_client: contract::Client<'a> =
        contract::Client::new(&e, &backstop_manager_address);
    (backstop_manager_address, backstop_manager_client)
}

/***** Env Utils *****/

pub const ONE_DAY_LEDGERS: u32 = 17280;
pub const PROTOCOL_VERSION: u32 = 27;

pub trait EnvTestUtils {
    /// Jump the env by the given amount of ledgers. Assumes 5 seconds per ledger.
    fn jump(&self, ledgers: u32);

    /// Set the ledger to the default LedgerInfo
    ///
    /// Time -> 1441065600 (Sept 1st, 2015 12:00:00 AM UTC)
    /// Sequence -> 100
    fn set_default_info(&self);
}

impl EnvTestUtils for Env {
    fn jump(&self, ledgers: u32) {
        self.ledger().set(LedgerInfo {
            timestamp: self.ledger().timestamp().saturating_add(ledgers as u64 * 5),
            protocol_version: PROTOCOL_VERSION,
            sequence_number: self.ledger().sequence().saturating_add(ledgers),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 120 * ONE_DAY_LEDGERS,
            min_persistent_entry_ttl: 120 * ONE_DAY_LEDGERS,
            max_entry_ttl: 365 * ONE_DAY_LEDGERS,
        });
    }

    fn set_default_info(&self) {
        self.ledger().set(LedgerInfo {
            timestamp: 1441065600, // Sept 1st, 2015 12:00:00 AM UTC
            protocol_version: PROTOCOL_VERSION,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 120 * ONE_DAY_LEDGERS,
            min_persistent_entry_ttl: 120 * ONE_DAY_LEDGERS,
            max_entry_ttl: 365 * ONE_DAY_LEDGERS,
        });
    }
}

/***** BLNT v3 Utils *****/

pub struct BlntFixture<'a> {
    pub blnt: StellarAssetClient<'a>,
    pub usdc: StellarAssetClient<'a>,
    pub xlm: StellarAssetClient<'a>,
    /// BLNT:XLM 80/20 Comet LP, used as the FirstLoss tier token
    pub blnt_xlm_lp: CometClient<'a>,
    /// BLNT:USDC 80/20 Comet LP, used as the SecondLoss tier token
    pub blnt_usdc_lp: CometClient<'a>,
    pub backstop: BackstopClient<'a>,
}

impl BlntFixture<'_> {
    /// Get the LP client for a tier of a pool created with `create_blnt_contracts`
    pub fn tier_lp(&self, tier: &BackstopTier) -> &CometClient<'_> {
        match tier {
            BackstopTier::FirstLoss => &self.blnt_xlm_lp,
            BackstopTier::SecondLoss => &self.blnt_usdc_lp,
            BackstopTier::ThirdLoss => panic!("no third tier configured"),
        }
    }
}

/// Deploy an 80/20 Comet LP between token_1 and token_2, seeded by admin with
/// 1,000 token_1 / 25 token_2 for 100 shares. Admin must be the SAC admin of both tokens.
fn create_comet_lp<'a>(
    e: &Env,
    admin: &Address,
    token_1: &StellarAssetClient,
    token_2: &StellarAssetClient,
) -> CometClient<'a> {
    let address = e.register(COMET_WASM, ());
    let client = CometClient::new(e, &address);
    token_1.mint(admin, &1_000_0000000);
    token_2.mint(admin, &25_0000000);
    client.init(
        admin,
        &vec![e, token_1.address.clone(), token_2.address.clone()],
        &vec![e, 0_8000000, 0_2000000],
        &vec![e, 1_000_0000000, 25_0000000],
        &0_0030000,
    );
    client
}

/// Mint `shares` of a Comet LP to `to`. Mints the required underlying tokens with generous slippage.
pub fn mint_lp(e: &Env, lp: &CometClient, to: &Address, shares: i128) {
    let tokens = lp.get_tokens();
    let mut max_in = vec![e];
    for token in tokens.iter() {
        // each share is backed by ~10 token_1 / 0.25 token_2, so 11 per share covers both legs
        let amount = shares * 11;
        StellarAssetClient::new(e, &token).mint(to, &amount);
        max_in.push_back(amount);
    }
    lp.join_pool(&shares, &max_in, to);
}

/// Deploy the BLNT v3 stack with a single pool configured like YieldBlent:
/// FirstLoss -> BLNT:XLM LP, SecondLoss -> BLNT:USDC LP.
///
/// Bombadil deposits into both tiers, the pool is added to the reward zone, and 7 days of
/// emissions are distributed. Requires `mock_all_auths`.
pub fn create_blnt_contracts<'a>(e: &Env, bombadil: &Address) -> (BlntFixture<'a>, Address) {
    let blnt = StellarAssetClient::new(
        e,
        &e.register_stellar_asset_contract_v2(bombadil.clone())
            .address(),
    );
    let usdc = StellarAssetClient::new(
        e,
        &e.register_stellar_asset_contract_v2(bombadil.clone())
            .address(),
    );
    let xlm = StellarAssetClient::new(
        e,
        &e.register_stellar_asset_contract_v2(bombadil.clone())
            .address(),
    );

    let blnt_usdc_lp = create_comet_lp(e, bombadil, &blnt, &usdc);
    let blnt_xlm_lp = create_comet_lp(e, bombadil, &blnt, &xlm);

    let backstop_id = Address::generate(e);
    let pool_factory_id = Address::generate(e);
    let incumbent_backstop = Address::generate(e);

    // emitter, initially pointing at a synthetic incumbent backstop
    let emitter_id = e.register(v3::emitter::WASM, (bombadil.clone(),));
    let emitter = v3::emitter::Client::new(e, &emitter_id);
    blnt.set_admin(&emitter_id);
    emitter.initialize(&blnt.address, &incumbent_backstop, &blnt_usdc_lp.address);

    let pool_hash = e.deployer().upload_contract_wasm(v3::pool::WASM);
    e.register_at(
        &pool_factory_id,
        v3::pool_factory::WASM,
        (v3::pool_factory::PoolInitMeta {
            backstop: backstop_id.clone(),
            pool_hash,
            blnt_id: blnt.address.clone(),
        },),
    );
    let pool_factory = v3::pool_factory::Client::new(e, &pool_factory_id);

    e.register_at(
        &backstop_id,
        BACKSTOP_WASM,
        (
            blnt_usdc_lp.address.clone(),
            blnt_xlm_lp.address.clone(),
            emitter_id.clone(),
            blnt.address.clone(),
            usdc.address.clone(),
            xlm.address.clone(),
            pool_factory_id.clone(),
            Vec::<(Address, i128)>::new(e),
        ),
    );
    let backstop = BackstopClient::new(e, &backstop_id);

    // swap the emitter to the v3 backstop. One LP base unit is enough to beat the
    // empty incumbent.
    blnt_usdc_lp.transfer(bombadil, &backstop_id, &1);
    let migration_start = e.ledger().timestamp();
    emitter.queue_swap_backstop(&backstop_id, &blnt_xlm_lp.address);
    backstop.distribute();
    e.ledger()
        .set_timestamp(migration_start + 24 * 60 * 60 * 24);
    backstop.distribute();
    e.ledger()
        .set_timestamp(migration_start + 31 * 60 * 60 * 24);
    emitter.swap_backstop();
    backstop.distribute();

    // deploy a two-tier pool
    let pool = pool_factory.deploy(
        bombadil,
        &String::from_str(e, "test"),
        &BytesN::<32>::random(e),
        &Address::generate(e),
        &0_1000000,
        &2,
        &0,
        &vec![
            e,
            v3::pool_factory::BackstopTierConfig {
                asset: v3::pool_factory::BackstopAsset::BlntXlm,
                take_rate_weight: 1,
            },
            v3::pool_factory::BackstopTierConfig {
                asset: v3::pool_factory::BackstopAsset::BlntUsdc,
                take_rate_weight: 1,
            },
        ],
        &None,
    );

    // bombadil backstops both tiers
    mint_lp(e, &blnt_xlm_lp, bombadil, 10_000_0000000);
    mint_lp(e, &blnt_usdc_lp, bombadil, 10_000_0000000);
    backstop.deposit(&BackstopTier::FirstLoss, bombadil, &pool, &5_000_0000000);
    backstop.deposit(&BackstopTier::SecondLoss, bombadil, &pool, &5_000_0000000);
    backstop.add_reward(&pool, &None);

    // start emissions
    e.jump(ONE_DAY_LEDGERS);
    backstop.distribute();
    backstop.gulp_emissions(&pool);

    e.jump(7 * ONE_DAY_LEDGERS);

    // emit 7 days worth of emissions
    backstop.distribute();
    backstop.gulp_emissions(&pool);

    e.jump(1);

    (
        BlntFixture {
            blnt,
            usdc,
            xlm,
            blnt_xlm_lp,
            blnt_usdc_lp,
            backstop,
        },
        pool,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::EnvTestConfig;

    #[test]
    fn test_create_blnt_contracts() {
        let e = Env::new_with_config(EnvTestConfig {
            capture_snapshot_at_drop: false,
        });
        e.cost_estimate().budget().reset_unlimited();
        e.set_default_info();
        e.mock_all_auths();
        let bombadil = Address::generate(&e);
        let (contracts, pool) = create_blnt_contracts(&e, &bombadil);

        assert_eq!(
            contracts
                .backstop
                .backstop_token(&BackstopTier::FirstLoss, &pool),
            contracts.blnt_xlm_lp.address
        );
        assert_eq!(
            contracts
                .backstop
                .backstop_token(&BackstopTier::SecondLoss, &pool),
            contracts.blnt_usdc_lp.address
        );
        assert!(contracts.backstop.reward_zone().contains(&pool));
    }
}
