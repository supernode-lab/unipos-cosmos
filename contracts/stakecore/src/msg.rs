use crate::state::PRECISION;
use crate::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
pub use unipos::stakecore::msg::*;

#[cw_serde]
pub struct InstantiateMsg {
    pub token: String,
    pub admin: String,
    pub provider: String,
    pub lock_period: Uint128,
    pub staker_shares: Uint128,
    pub min_stake_amount: Uint128,
    pub apy: Uint128,
    pub installment_num: Uint128,
}

impl InstantiateMsg {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.lock_period == Uint128::zero() {
            return Err(ContractError::InvalidInput);
        }

        if self.staker_shares == Uint128::zero() || self.staker_shares > Uint128::from(PRECISION) {
            return Err(ContractError::InvalidInput);
        }

        if self.installment_num == Uint128::zero() {
            return Err(ContractError::InvalidInput);
        }

        Ok(())
    }
}
