use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Env, Uint128, Uint256, Uint64};

pub const UNIPOS_YEAR: u128 = 360 * 86400;
pub const PRECISION: u128 = 1e18 as u128;

#[cw_serde]
pub struct Config {
    pub lock_period: Uint64,
    pub cliff_period: Uint64,
    pub apy: Uint128,
    pub installment_num: Uint128,
    pub min_stake_amount: Uint128,
    pub enable_staker_whitelist: bool,
    pub enable_beneficiary_whitelist: bool,
}

impl Config {
    pub fn calc_security_deposit_by_collateral(&self, amount: Uint128) -> Uint128 {
        ((Uint256::from(amount)
            * ((Uint256::from(self.apy) * Uint256::from(self.lock_period))
                / Uint256::from(UNIPOS_YEAR)))
            / (Uint256::from(PRECISION)))
        .try_into()
        .unwrap()
    }

    pub fn calc_unlocked_installment_reward(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let now = Uint64::from(env.block.time.seconds());
        let elapsed_t = if now <= stake_info.start_time {
            Uint64::zero()
        } else {
            now - stake_info.start_time
        };

        if elapsed_t >= self.lock_period {
            stake_info.total_rewards
        } else {
            stake_info.total_rewards
                * (Uint128::from(elapsed_t) * self.installment_num
                    / Uint128::from(self.lock_period))
                / self.installment_num
        }
    }

    pub fn calc_unlocked_installment_principal(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let now = Uint64::from(env.block.time.seconds());
        let elapsed_t = if now <= stake_info.start_time {
            Uint64::zero()
        } else {
            now - stake_info.start_time
        };

        if elapsed_t >= self.lock_period {
            return stake_info.total_principal;
        }

        Uint128::zero()
    }

    pub fn calc_withdrawable_reward(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let unlocked_reward = self.calc_unlocked_installment_reward(env, stake_info);
        if unlocked_reward <= stake_info.withdrawn_rewards {
            return Uint128::zero();
        }

        unlocked_reward - stake_info.withdrawn_rewards
    }

    pub fn calc_withdrawable_principal(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let unlocked_principal = self.calc_unlocked_installment_principal(env, stake_info);
        if unlocked_principal <= stake_info.withdrawn_principal {
            return Uint128::zero();
        }

        unlocked_principal - stake_info.withdrawn_principal
    }
}

#[derive(Default)]
#[cw_serde]
pub struct AssetInfo {
    pub total_principal: Uint128,
    pub withdrawn_principal: Uint128,
    pub total_rewards: Uint128,
    pub withdrawn_rewards: Uint128,
    pub total_security_deposit: Uint128,
}

#[cw_serde]
pub struct StakeInfo {
    pub owner: Addr,
    pub start_time: Uint64,
    pub total_principal: Uint128,
    pub withdrawn_principal: Uint128,
    pub total_rewards: Uint128,
    pub withdrawn_rewards: Uint128,
}
