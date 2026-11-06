use soroban_sdk::{contracttype, unwrap::UnwrapOptimized, Address, Env, Symbol, Vec};

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

/********** Ledger Thresholds **********/

const ONE_DAY_LEDGERS: u32 = 17280; // assumes 5 seconds per ledger

const LEDGER_BUMP: u32 = 120 * ONE_DAY_LEDGERS;
const LEDGER_THRESHOLD: u32 = LEDGER_BUMP - 20 * ONE_DAY_LEDGERS;

/********** Ledger Keys **********/

const OWNER_KEY: &str = "Owner";
const MANAGER_KEY: &str = "Manager";
const BACKSTOPS_KEY: &str = "Bstop";
const POOLS_KEY: &str = "Pools";

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
