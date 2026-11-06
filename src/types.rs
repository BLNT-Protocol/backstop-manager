use soroban_sdk::contracttype;

use crate::dependencies::backstop;

/// A v3 backstop loss-waterfall position. Mirrors the backstop's `BackstopTier` so the
/// type is included in this contract's spec.
#[contracttype]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackstopTier {
    FirstLoss,
    SecondLoss,
    ThirdLoss,
}

impl From<BackstopTier> for backstop::BackstopTier {
    fn from(tier: BackstopTier) -> Self {
        match tier {
            BackstopTier::FirstLoss => backstop::BackstopTier::FirstLoss,
            BackstopTier::SecondLoss => backstop::BackstopTier::SecondLoss,
            BackstopTier::ThirdLoss => backstop::BackstopTier::ThirdLoss,
        }
    }
}
