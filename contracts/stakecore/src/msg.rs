#[allow(unused_imports)]
use crate::types::{AssetInfo, Config, StakeInfo};
pub use universal_token::TokenConfig;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Uint64};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub providers: Vec<String>,
    pub is_native_token: bool,
    pub token: String,
    pub lock_period: Uint64,
    pub cliff_period: Uint64,
    pub apy: Uint128,
    pub installment_num: Uint128,
    pub min_stake_amount: Uint128,
    pub enable_staker_whitelist: bool,
    pub enable_beneficiary_whitelist: bool,
}



#[cw_serde]
pub enum ExecuteMsg {
    DepositSecurity {
        amount: Uint128,
    },
    WithdrawSecurity {
        amount: Uint128,
    },
    Stake {
        owner: String,
        amount: Uint128,
    },
    WithdrawPrincipal {
        index: u32,
    },
    WithdrawRewards {
        index: u32,
    },
    WithdrawRewardsBatch {
        indexes: Vec<u32>,
    },
    Collect {
        is_native_token: bool,
        token: String,
    },

    GrantRole {
        role: String,
        account: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config,

    #[returns(AssetInfo)]
    AssetInfo,

    #[returns(StakeInfo)]
    StakeInfo { index: u32 },

    #[returns(StakerIndexesResponse)]
    StakerIndexes { account: String },

    #[returns(TokenConfig)]
    Token,
    
    #[returns[bool]]
    HaveRole{role: String, account: String},
}

#[cw_serde]
pub struct StakerIndexesResponse {
    pub indexes: Vec<u32>,
}
