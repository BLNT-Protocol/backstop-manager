use soroban_sdk::{contractevent, Address};

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
