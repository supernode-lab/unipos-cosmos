use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128, Uint256, Uint64};

#[cw_serde]
pub struct Config {
    pub stakecore: Option<Addr>,
    pub enable_shareholder_whitelist: bool,
}

#[cw_serde]
pub struct ShareInfo {
    pub start_time: Uint64,
    pub recycled_time: Uint64,
    pub end_time: Uint64,

    pub total_rewards: Uint128,
    pub claimed_rewards: Uint128,
    pub withdrawn_rewards: Uint128,
    pub granted_rewards: Uint128,

    pub total_recycled_rewards: Uint128,
    pub withdrawn_recycled_rewards: Uint128,

    pub total_principal: Uint128,
    pub claimed_principal: Uint128,
    pub withdrawn_principal: Uint128,
    pub granted_principal: Uint128,
}

impl ShareInfo {
    pub fn ungranted_rewards(&self) -> Uint128 {
        self.total_rewards - self.granted_rewards - self.total_recycled_rewards
    }

    pub fn rewards_balance(&self) -> Uint128 {
        self.claimed_rewards - self.withdrawn_rewards
    }

    pub fn principal_balance(&self) -> Uint128 {
        self.total_principal - self.withdrawn_principal
    }

    pub fn recyclable_rewards(&self, recycling_time: Uint64) -> Uint128 {
        Uint128::try_from(
            Uint256::from(self.ungranted_rewards())
                * Uint256::from(recycling_time - self.recycled_time)
                / Uint256::from(self.end_time - self.recycled_time),
        )
        .unwrap()
    }

    pub fn calc_shareholder_unlocked_principal(
        &self,
        shareholder_info: &ShareholderInfo,
    ) -> Uint128 {
        if self.total_principal == Uint128::zero() || self.claimed_principal == Uint128::zero() {
            return Uint128::zero();
        }

        (Uint256::from(shareholder_info.granted_principal) * Uint256::from(self.claimed_principal)
            / Uint256::from(self.total_principal))
        .try_into()
        .unwrap()
    }

    pub fn calc_shareholder_withdrawable_principal(
        &self,
        shareholder_info: &ShareholderInfo,
    ) -> Uint128 {
        let unlocked_principal = self.calc_shareholder_unlocked_principal(&shareholder_info);
        if unlocked_principal > shareholder_info.withdrawn_principal {
            unlocked_principal - shareholder_info.withdrawn_principal
        } else {
            Uint128::zero()
        }
    }

    pub fn calc_shareholder_withdrawable_rewards(
        &self,
        shareholder_info: &ShareholderInfo,
    ) -> Uint128 {
        let unlocked_rewards = self.calc_shareholder_unlocked_rewards(shareholder_info);
        if unlocked_rewards > shareholder_info.withdrawn_rewards {
            unlocked_rewards - shareholder_info.withdrawn_rewards
        } else {
            Uint128::zero()
        }
    }

    pub fn calc_shareholder_unlocked_rewards(&self, shareholder_info: &ShareholderInfo) -> Uint128 {
        if self.total_rewards == Uint128::zero() || self.claimed_rewards == Uint128::zero() {
            return Uint128::zero();
        }

        let gross =
            Uint256::from(shareholder_info.pre_recycled_rewards + shareholder_info.granted_rewards)
                * Uint256::from(self.claimed_rewards)
                / Uint256::from(self.total_rewards);

        let gross = Uint128::try_from(gross).unwrap();
        if gross > shareholder_info.pre_recycled_rewards {
            gross - shareholder_info.pre_recycled_rewards
        } else {
            Uint128::zero()
        }
    }
}

#[cw_serde]
pub struct ShareholderInfo {
    pub pre_recycled_rewards: Uint128,
    pub granted_rewards: Uint128,
    pub withdrawn_rewards: Uint128,
    pub granted_principal: Uint128,
    pub withdrawn_principal: Uint128,
}
