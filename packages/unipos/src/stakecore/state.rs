use crate::Ownership;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Env, Uint128, Uint256};

pub const UNIPOS_YEAR: u128 = 360 * 86400;
pub const PRECISION: u128 = 1e18 as u128;

#[cw_serde]
pub struct Config {
    pub token: Addr,
    pub lock_period: Uint128,
    pub staker_reward_share: Uint128,
    pub apy: Uint128,
    pub min_stake_amount: Uint128,
    pub installment_num: Uint128,
    pub provider: Ownership,
    pub admin: Ownership,
}

impl Config {
    pub fn calc_security_deposit_by_collateral(&self, amount: Uint128) -> Uint128 {
        ((Uint256::from(amount) * Uint256::from(self.apy) * Uint256::from(self.lock_period))
            / (Uint256::from(PRECISION * UNIPOS_YEAR)))
        .try_into()
        .unwrap()
    }

    pub fn calc_staker_reward(&self, amount: Uint128) -> Uint128 {
        let total_reward = self.calc_security_deposit_by_collateral(amount);
        (Uint256::from(total_reward) * Uint256::from(self.staker_reward_share)
            / Uint256::from(PRECISION))
        .try_into()
        .unwrap()
    }

    pub fn calc_unlocked_installment_reward(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let now = Uint128::from(env.block.time.seconds());
        let elapsed_t = if now <= stake_info.start_at {
            Uint128::zero()
        } else {
            now - stake_info.start_at
        };

        if elapsed_t >= self.lock_period {
            stake_info.total_reward
        } else {
            stake_info.total_reward * (elapsed_t * self.installment_num / self.lock_period)
                / self.installment_num
        }
    }

    pub fn calc_claimable_reward(&self, env: &Env, stake_info: &StakeInfo) -> Uint128 {
        let unlocked_reward = self.calc_unlocked_installment_reward(env, stake_info);
        unlocked_reward - stake_info.claimed_reward
    }

    pub fn calc_beneficiary_reward_by_staker_reward(&self, amount: Uint128) -> Uint128 {
        (Uint256::from(amount) * Uint256::from(Uint128::from(PRECISION) - self.staker_reward_share)
            / Uint256::from(self.staker_reward_share))
        .try_into()
        .unwrap()
    }
}

#[derive(Default)]
#[cw_serde]
pub struct BeneficiaryInfo {
    pub beneficiary: Option<Addr>,
    pub total_reward: Uint128,
    pub claimed_reward: Uint128,
}

#[derive(Default)]
#[cw_serde]
pub struct AssetInfo {
    pub total_collateral: Uint128,
    pub unstaked_collateral: Uint128,
    pub total_claimed_reward: Uint128,
    pub total_security_deposit: Uint128,
}

#[cw_serde]
pub struct StakeInfo {
    pub owner: Addr,
    pub amount: Uint128,
    pub start_at: Uint128,
    pub total_reward: Uint128,
    pub claimed_reward: Uint128,
    pub unstaked: bool,
}
