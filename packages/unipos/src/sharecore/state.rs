use crate::Ownership;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256};

#[cw_serde]
pub struct Config {
    pub stake_core: Option<Addr>,
    pub token: Option<Addr>,
    pub admin: Ownership,
}

// Invariants: granted_reward ≤ total_reward, granted_principal ≤ principal
#[cw_serde]
pub struct ShareInfo {
    pub total_reward: Uint128,
    pub claimed_reward: Uint128,
    pub granted_reward: Uint128,
    pub principal: Uint128,
    pub claimed_principal: Uint128,
    pub granted_principal: Uint128,
}

impl ShareInfo {
    pub fn calc_holder_reward(&self, granted_reward: Uint128) -> Uint128 {
        if granted_reward.is_zero() || self.total_reward.is_zero() {
            return Uint128::zero();
        }

        (Uint256::from(granted_reward) * Uint256::from(self.claimed_reward)
            / Uint256::from(self.total_reward))
        .try_into()
        .unwrap()
    }

    pub fn calc_holder_principal(&self, granted_principal: Uint128) -> Uint128 {
        if granted_principal.is_zero() || self.principal.is_zero() {
            return Uint128::zero();
        }

        (Uint256::from(granted_principal) * Uint256::from(self.claimed_principal)
            / Uint256::from(self.principal))
        .try_into()
        .unwrap()
    }
}

#[cw_serde]
pub struct HolderInfo {
    pub granted_reward: Uint128,
    pub claimed_reward: Uint128,
    pub granted_principal: Uint128,
    pub claimed_principal: Uint128,
}
