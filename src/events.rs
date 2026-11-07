use soroban_sdk::{contractevent, Address};

use crate::types::BackstopTier;

/// Emitted when the owner proposes a new owner
#[contractevent(data_format = "single-value")]
pub struct OwnerProposed {
    #[topic]
    pub owner: Address,
    pub new_owner: Address,
}

/// Emitted when the pending owner accepts ownership
#[contractevent(data_format = "single-value")]
pub struct OwnerAccepted {
    #[topic]
    pub new_owner: Address,
    pub old_owner: Address,
}

/// Emitted when the owner cancels, or the pending owner declines, a proposal
#[contractevent(data_format = "single-value")]
pub struct OwnerCancelled {
    #[topic]
    pub caller: Address,
    pub new_owner: Address,
}

/// Emitted when the owner sets the manager's share of earnings
#[contractevent(data_format = "single-value")]
pub struct HarvestBpsSet {
    pub bps: u32,
}

/// Emitted when the owner sets the principal of a position
#[contractevent(data_format = "vec")]
pub struct PrincipalSet {
    #[topic]
    pub backstop: Address,
    #[topic]
    pub pool: Address,
    pub tier: BackstopTier,
    pub amount: i128,
}

/// Emitted when the owner sets the amount of a token owed to the manager
#[contractevent(data_format = "single-value")]
pub struct HarvestableSet {
    #[topic]
    pub token: Address,
    pub amount: i128,
}

/// Emitted when the manager is paid its harvestable balance of a token
#[contractevent(data_format = "single-value")]
pub struct Harvest {
    #[topic]
    pub token: Address,
    #[topic]
    pub manager: Address,
    pub amount: i128,
}

/// Emitted by `b_withdraw` with the split of the withdrawn tokens
#[contractevent(data_format = "vec")]
pub struct EarningsRealized {
    #[topic]
    pub backstop: Address,
    #[topic]
    pub pool: Address,
    pub tier: BackstopTier,
    pub principal_part: i128,
    pub earnings_part: i128,
    pub manager_part: i128,
}
