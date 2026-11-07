use soroban_sdk::{contracttype, unwrap::UnwrapOptimized, Address, Env, Symbol, Vec};

use crate::types::BackstopTier;

/********** Storage Types **********/

#[contracttype]
pub struct Manager {
    /// The address of the manager
    pub id: Address,
    /// The scope of the manager
    /// 0 -> Low
    /// 1 -> Medium
    /// 2 -> High
    pub scope: u32,
}

/// A proposed ownership transfer awaiting acceptance
#[contracttype]
pub struct PendingOwner {
    /// The proposed owner
    pub id: Address,
    /// The ledger sequence the proposal was made in
    pub ledger: u32,
    /// The ledger timestamp the proposal was made at
    pub timestamp: u64,
}

/// Identifies a backstop position held by the contract
#[contracttype]
#[derive(Clone)]
pub struct PositionKey {
    pub backstop: Address,
    pub pool: Address,
    pub tier: BackstopTier,
}

/// Persistent storage keys
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    /// LP tokens deposited into a position that have not been withdrawn
    Principal(PositionKey),
    /// LP tokens of a token owed to the manager
    Harvest(Address),
}

/********** Ledger Thresholds **********/

const ONE_DAY_LEDGERS: u32 = 17280; // assumes 5 seconds per ledger

const LEDGER_BUMP: u32 = 120 * ONE_DAY_LEDGERS;
const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - 20 * ONE_DAY_LEDGERS;

/********** Ledger Keys **********/

const OWNER_KEY: &str = "Owner";
const MANAGER_KEY: &str = "Manager";
const BACKSTOPS_KEY: &str = "Bstop";
const POOLS_KEY: &str = "Pools";
const PENDING_OWNER_KEY: &str = "PendOwner";
const HARVEST_BPS_KEY: &str = "HarvBps";

/********** Ledger Thresholds **********/

/// Bump the instance lifetime by the defined amount
pub fn extend_instance(e: &Env) {
    e.storage()
        .instance()
        .extend_ttl(LEDGER_THRESHOLD, LEDGER_BUMP);
}

/********** Instance **********/

/// Get the owner address
pub fn get_owner(e: &Env) -> Address {
    e.storage()
        .instance()
        .get::<Symbol, Address>(&Symbol::new(e, OWNER_KEY))
        .unwrap_optimized()
}

/// Set the owner address
pub fn set_owner(e: &Env, owner: &Address) {
    e.storage()
        .instance()
        .set::<Symbol, Address>(&Symbol::new(e, OWNER_KEY), owner);
}

/// Get the pending owner proposal, if one exists
pub fn get_pending_owner(e: &Env) -> Option<PendingOwner> {
    e.storage()
        .instance()
        .get::<Symbol, PendingOwner>(&Symbol::new(e, PENDING_OWNER_KEY))
}

/// Set the pending owner proposal
pub fn set_pending_owner(e: &Env, pending: &PendingOwner) {
    e.storage()
        .instance()
        .set::<Symbol, PendingOwner>(&Symbol::new(e, PENDING_OWNER_KEY), pending);
}

/// Remove the pending owner proposal
pub fn del_pending_owner(e: &Env) {
    e.storage()
        .instance()
        .remove::<Symbol>(&Symbol::new(e, PENDING_OWNER_KEY));
}

/// Get the manager for the contract
pub fn get_manager(e: &Env) -> Manager {
    e.storage()
        .instance()
        .get::<Symbol, Manager>(&Symbol::new(e, MANAGER_KEY))
        .unwrap_optimized()
}

/// Set the manager for the contract
pub fn set_manager(e: &Env, manager: &Manager) {
    e.storage()
        .instance()
        .set::<Symbol, Manager>(&Symbol::new(e, MANAGER_KEY), manager);
}

/// Get an array of all valid backstops the vault can interact with
pub fn get_valid_backstops(e: &Env) -> Vec<Address> {
    let key = Symbol::new(e, BACKSTOPS_KEY);
    e.storage()
        .instance()
        .get::<Symbol, Vec<Address>>(&key)
        .unwrap_optimized()
}

/// Set the valid backstop addresses
pub fn set_valid_backstops(e: &Env, backstop: &Vec<Address>) {
    let key = Symbol::new(e, BACKSTOPS_KEY);
    e.storage()
        .instance()
        .set::<Symbol, Vec<Address>>(&key, backstop);
}

/// Get an array of all valid pools the vault can interact with
pub fn get_valid_pools(e: &Env) -> Vec<Address> {
    let key = Symbol::new(e, POOLS_KEY);
    e.storage()
        .instance()
        .get::<Symbol, Vec<Address>>(&key)
        .unwrap_optimized()
}

/// Set the valid pool addresses
pub fn set_valid_pools(e: &Env, pools: &Vec<Address>) {
    let key = Symbol::new(e, POOLS_KEY);
    e.storage()
        .instance()
        .set::<Symbol, Vec<Address>>(&key, pools);
}

/// Get the manager's share of earnings, in basis points. Defaults to 0.
pub fn get_harvest_bps(e: &Env) -> u32 {
    e.storage()
        .instance()
        .get::<Symbol, u32>(&Symbol::new(e, HARVEST_BPS_KEY))
        .unwrap_or(0)
}

/// Set the manager's share of earnings, in basis points
pub fn set_harvest_bps(e: &Env, bps: u32) {
    e.storage()
        .instance()
        .set::<Symbol, u32>(&Symbol::new(e, HARVEST_BPS_KEY), &bps);
}

/********** Persistent **********/

fn get_persistent_i128(e: &Env, key: &DataKey) -> i128 {
    let storage = e.storage().persistent();
    match storage.get::<DataKey, i128>(key) {
        Some(value) => {
            storage.extend_ttl(key, LEDGER_THRESHOLD, LEDGER_BUMP);
            value
        }
        None => 0,
    }
}

fn set_persistent_i128(e: &Env, key: &DataKey, value: i128) {
    let storage = e.storage().persistent();
    storage.set::<DataKey, i128>(key, &value);
    storage.extend_ttl(key, LEDGER_THRESHOLD, LEDGER_BUMP);
}

/// Get the principal of a position, in LP tokens. Defaults to 0.
pub fn get_principal(e: &Env, position: &PositionKey) -> i128 {
    get_persistent_i128(e, &DataKey::Principal(position.clone()))
}

/// Set the principal of a position, in LP tokens
pub fn set_principal(e: &Env, position: &PositionKey, amount: i128) {
    set_persistent_i128(e, &DataKey::Principal(position.clone()), amount);
}

/// Get the amount of a token owed to the manager. Defaults to 0.
pub fn get_harvestable(e: &Env, token: &Address) -> i128 {
    get_persistent_i128(e, &DataKey::Harvest(token.clone()))
}

/// Set the amount of a token owed to the manager
pub fn set_harvestable(e: &Env, token: &Address, amount: i128) {
    set_persistent_i128(e, &DataKey::Harvest(token.clone()), amount);
}
