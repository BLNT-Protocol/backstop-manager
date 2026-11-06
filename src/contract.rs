use crate::{
    dependencies::{
        backstop::{self, Client as BackstopClient},
        comet::Client as CometClient,
    },
    errors::BackstopManagerErrors,
    storage::{self, Manager},
    types::BackstopTier,
};
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contractimpl, panic_with_error,
    token::TokenClient,
    unwrap::UnwrapOptimized,
    vec, Address, Env, IntoVal, Symbol, Vec,
};

const MAX_VALID_LIST_LEN: u32 = 4;

#[contract]
pub struct BackstopManager;

#[contractimpl]
impl BackstopManager {
    /********** Constructor **********/

    /// Initialize the backstop manager
    ///
    /// ### Arguments
    /// * owner - The address of the owner of the funds
    /// * manager - The address of the manager of the funds
    /// * admin_scope - The scope of the manager. 0 = Low, 1 = Medium, 2 = High
    /// * backstops - The addresses of the backstops the manager can interact with initially
    /// * pools - The addresses of the pools the manager can interact with initially
    ///
    /// ### Errors
    /// * InvalidScope - The scope is greater than 2
    /// * ContractListOverMax - Too many backstops or pools were provided
    pub fn __constructor(
        e: Env,
        owner: Address,
        manager: Address,
        admin_scope: u32,
        backstops: Vec<Address>,
        pools: Vec<Address>,
    ) {
        storage::set_owner(&e, &owner);
        if admin_scope > 2 {
            panic_with_error!(&e, BackstopManagerErrors::InvalidScope);
        }
        storage::set_manager(
            &e,
            &Manager {
                id: manager,
                scope: admin_scope,
            },
        );

        if backstops.len() > MAX_VALID_LIST_LEN {
            panic_with_error!(&e, BackstopManagerErrors::ContractListOverMax);
        }
        if pools.len() > MAX_VALID_LIST_LEN {
            panic_with_error!(&e, BackstopManagerErrors::ContractListOverMax);
        }
        storage::set_valid_backstops(&e, &backstops);
        storage::set_valid_pools(&e, &pools);
        storage::extend_instance(&e);
    }

    /********** Read-Only **********/

    /// Get owner
    pub fn owner(e: Env) -> Address {
        storage::get_owner(&e)
    }

    /// Get manager
    pub fn manager(e: Env) -> Manager {
        storage::get_manager(&e)
    }

    /// Get the backstops this contract can interact with
    pub fn backstops(e: Env) -> Vec<Address> {
        storage::get_valid_backstops(&e)
    }

    /// Get the pools this contract can interact with
    pub fn pools(e: Env) -> Vec<Address> {
        storage::get_valid_pools(&e)
    }

    /********** Owner **********/

    /// (Only Owner) Transfer tokens from the manager contract to another address
    ///
    /// ### Arguments
    /// * `token` - The address of the token to transfer
    /// * `to` - The address to transfer the tokens to
    /// * `amount` - The amount of tokens to transfer
    pub fn transfer_token(e: Env, token: Address, to: Address, amount: i128) {
        let owner = storage::get_owner(&e);
        owner.require_auth();
        storage::extend_instance(&e);

        let token_client = TokenClient::new(&e, &token);
        token_client.transfer(&e.current_contract_address(), &to, &amount);
    }

    /// (Only Owner) Set the manager for the contract
    ///
    /// ### Arguments
    /// * `manager` - The address of the manager
    /// * `scope` - The scope of the manager. 0 = Low, 1 = Medium, 2 = High
    pub fn set_manager(e: Env, manager: Address, scope: u32) {
        let owner = storage::get_owner(&e);
        owner.require_auth();
        storage::extend_instance(&e);

        if scope > 2 {
            panic_with_error!(&e, BackstopManagerErrors::InvalidScope);
        }

        storage::set_manager(&e, &Manager { id: manager, scope });
    }

    /// (Only Owner) Set the list of valid backstops
    ///
    /// ### Arguments
    /// * `backstops` - The addresses of the backstops the manager can interact with
    pub fn set_backstops(e: Env, backstops: Vec<Address>) {
        let owner = storage::get_owner(&e);
        owner.require_auth();
        storage::extend_instance(&e);
        if backstops.len() > MAX_VALID_LIST_LEN {
            panic_with_error!(&e, BackstopManagerErrors::ContractListOverMax);
        }
        storage::set_valid_backstops(&e, &backstops);
    }

    /// (Only Owner) Set the list of valid pools
    ///
    /// ### Arguments
    /// * `pools` - The addresses of the pools the manager can interact with
    pub fn set_pools(e: Env, pools: Vec<Address>) {
        let owner = storage::get_owner(&e);
        owner.require_auth();
        storage::extend_instance(&e);
        if pools.len() > MAX_VALID_LIST_LEN {
            panic_with_error!(&e, BackstopManagerErrors::ContractListOverMax);
        }
        storage::set_valid_pools(&e, &pools);
    }

    /********** Manager **********/

    /// (Manager, Low) Transfer tokens from the contract back to the owner
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `token` - The address of the token to transfer
    /// * `amount` - The amount of tokens to transfer
    pub fn refund_token(e: Env, from: Address, token: Address, amount: i128) {
        require_auth_with_scope(&e, from, 0);
        storage::extend_instance(&e);

        let owner = storage::get_owner(&e);
        let token_client = TokenClient::new(&e, &token);
        token_client.transfer(&e.current_contract_address(), &owner, &amount);
    }

    /***** Backstop Interactions *****/

    /// (Manager, Low) Claim backstop deposit emissions for one tier of a pool for the contract
    ///
    /// Returns the amount of backstop tokens minted from the claimed BLNT emissions
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier to claim emissions for
    /// * `pool_address` - The address of the pool to claim from
    /// * `min_lp_tokens_out` - The minimum amount of backstop tokens to mint with emissions
    ///
    /// ### Errors
    /// If an invalid backstop or pool address is included
    pub fn b_claim(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        min_lp_tokens_out: i128,
    ) -> i128 {
        require_auth_with_scope(&e, from, 0);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        BackstopClient::new(&e, &backstop).claim(
            &tier,
            &e.current_contract_address(),
            &vec![&e, pool_address],
            &min_lp_tokens_out,
        )
    }

    /// (Manager, Medium) Deposit "amount" of a tier's backstop token from the contract into the backstop for "pool_address"
    ///
    /// Returns the number of backstop pool shares minted
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier to deposit into
    /// * `pool_address` - The address of the pool
    /// * `amount` - The amount of tokens to deposit
    pub fn b_deposit(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) -> i128 {
        require_auth_with_scope(&e, from, 1);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        let backstop_client = BackstopClient::new(&e, &backstop);
        let tier_token = backstop_client.backstop_token(&tier, &pool_address);
        e.authorize_as_current_contract(vec![
            &e,
            InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: tier_token,
                    fn_name: Symbol::new(&e, "transfer"),
                    args: vec![
                        &e,
                        e.current_contract_address().into_val(&e),
                        backstop.into_val(&e),
                        amount.into_val(&e),
                    ],
                },
                sub_invocations: vec![&e],
            }),
        ]);
        backstop_client.deposit(&tier, &e.current_contract_address(), &pool_address, &amount)
    }

    /// (Manager, Medium) Queue deposited pool shares from the contract for withdraw from a tier of a pool's backstop
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier to queue shares from
    /// * `pool_address` - The address of the pool
    /// * `amount` - The amount of shares to queue for withdraw
    pub fn b_queue_withdrawal(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) {
        require_auth_with_scope(&e, from, 1);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        BackstopClient::new(&e, &backstop).queue_withdrawal(
            &tier,
            &e.current_contract_address(),
            &pool_address,
            &amount,
        );
    }

    /// (Manager, Medium) Dequeue a currently queued pool share withdraw for the contract from a tier of a pool's backstop
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier to dequeue shares from
    /// * `pool_address` - The address of the pool
    /// * `amount` - The amount of shares to dequeue
    pub fn b_dequeue_withdrawal(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) {
        require_auth_with_scope(&e, from, 1);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        BackstopClient::new(&e, &backstop).dequeue_withdrawal(
            &tier,
            &e.current_contract_address(),
            &pool_address,
            &amount,
        )
    }

    /// (Manager, High) Withdraw shares from the contract's withdraw queue for a tier of a pool's backstop
    ///
    /// The withdrawn tokens are always sent to this contract.
    ///
    /// Returns the amount of tokens returned
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier to withdraw from
    /// * `pool_address` - The address of the pool
    /// * `amount` - The amount of shares to withdraw
    pub fn b_withdraw(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        amount: i128,
    ) -> i128 {
        require_auth_with_scope(&e, from, 2);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        BackstopClient::new(&e, &backstop).withdraw(
            &tier,
            &e.current_contract_address(),
            &pool_address,
            &amount,
            &e.current_contract_address(),
        )
    }

    /***** Backstop Token Interactions *****/

    /// (Manager, High) Join the Comet LP used as the backstop token for a tier of a pool.
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier whose token is the Comet LP to join
    /// * `pool_address` - The address of the pool
    /// * `pool_amount_out` - The amount of pool shares to mint
    /// * `max_amounts_in` - The maximum amount of tokens to deposit
    ///
    /// ### Errors
    /// If an invalid backstop or pool address is included
    pub fn c_join_pool(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        pool_amount_out: i128,
        max_amounts_in: Vec<i128>,
    ) {
        require_auth_with_scope(&e, from, 2);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        let backstop_token =
            BackstopClient::new(&e, &backstop).backstop_token(&tier, &pool_address);
        let comet = CometClient::new(&e, &backstop_token);
        let comet_tokens = comet.get_tokens();
        let mut auths = vec![&e];
        for index in 0..comet_tokens.len() {
            let amount = max_amounts_in.get(index).unwrap_optimized();
            let token_address = comet_tokens.get(index).unwrap_optimized();
            let approval_ledger = (e.ledger().sequence() / 100000 + 1) * 100000;
            auths.push_back(InvokerContractAuthEntry::Contract(SubContractInvocation {
                context: ContractContext {
                    contract: token_address,
                    fn_name: Symbol::new(&e, "approve"),
                    args: vec![
                        &e,
                        e.current_contract_address().into_val(&e),
                        backstop_token.into_val(&e),
                        amount.into_val(&e),
                        approval_ledger.into_val(&e),
                    ],
                },
                sub_invocations: vec![&e],
            }));
        }
        e.authorize_as_current_contract(auths);
        comet.join_pool(
            &pool_amount_out,
            &max_amounts_in,
            &e.current_contract_address(),
        );
    }

    /// (Manager, High) Exit the Comet LP used as the backstop token for a tier of a pool.
    ///
    /// ### Arguments
    /// * `from` - The caller of the function
    /// * `backstop` - The address of the backstop contract
    /// * `tier` - The backstop tier whose token is the Comet LP to exit
    /// * `pool_address` - The address of the pool
    /// * `burn_amount` - The amount of pool shares to burn
    /// * `min_amounts_out` - The minimum amount of tokens to receive
    ///
    /// ### Errors
    /// If an invalid backstop or pool address is included
    pub fn c_exit_pool(
        e: Env,
        from: Address,
        backstop: Address,
        tier: BackstopTier,
        pool_address: Address,
        burn_amount: i128,
        min_amounts_out: Vec<i128>,
    ) {
        require_auth_with_scope(&e, from, 2);
        require_backstop_and_pool_valid(&e, &backstop, &pool_address);
        storage::extend_instance(&e);
        let tier: backstop::BackstopTier = tier.into();

        let backstop_token =
            BackstopClient::new(&e, &backstop).backstop_token(&tier, &pool_address);
        let comet = CometClient::new(&e, &backstop_token);
        comet.exit_pool(
            &burn_amount,
            &min_amounts_out,
            &e.current_contract_address(),
        )
    }
}

/// Authorize an action based on a provide scope for from. If `from` is the owner,
/// then the action is authorized. If `from` is the manager, then the manager is validated
/// to have the appropriate scope.
///
/// THIS CALLS REQUIRE AUTH FOR FROM
///
/// ### Arguments
/// * `from` - The address of the caller
/// * `scope` - The scope required for the action
///
/// ### Errors
/// * UnauthorizedError - The caller is not authorized to perform the action
fn require_auth_with_scope(e: &Env, from: Address, scope: u32) {
    from.require_auth();
    if from == storage::get_owner(e) {
        return;
    }
    let manager = storage::get_manager(e);
    if manager.id != from || manager.scope < scope {
        panic_with_error!(&e, BackstopManagerErrors::UnauthorizedError);
    }
}

/// Validate that the backstop and pool address are included in the valid lists
///
/// ### Arguments
/// * `backstop` - The address of the backstop contract
/// * `pool_address` - The address of the pool
///
/// ### Errors
/// * InvalidContractAddress - The backstop or pool address is not included in the valid lists
fn require_backstop_and_pool_valid(e: &Env, backstop: &Address, pool_address: &Address) {
    let backstops = storage::get_valid_backstops(e);
    let pools = storage::get_valid_pools(e);
    if !backstops.contains(backstop) || !pools.contains(pool_address) {
        panic_with_error!(&e, BackstopManagerErrors::InvalidContractAddress);
    }
}
