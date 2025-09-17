#![allow(unused_imports)]
use crate::types::*;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128, Uint64};
use universal_token::TokenConfig;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub stakecore: Option<String>,
    pub is_native_token: bool,
    pub token: String,
    pub enable_shareholder_whitelist: bool,
}

#[cw_serde]
pub enum ExecuteMsg {
    InitStakeCore {
        stakecore: String,
    },

    AccrueRewards {
        share_id: u32,
        recycled_time: Uint64,
    },

    Recycle {
        share_id: u32,
        amount: Uint128,
    },

    AddShareholder {
        owner: String,
        share_id: u32,
        granted_reward: Uint128,
        granted_principal: Uint128,
    },

    AddShareholderWithTime {
        owner: String,
        share_id: u32,
        start_time: Uint64,
        granted_reward: Uint128,
        granted_principal: Uint128,
    },

    WithdrawRewards {
        share_id: u32,
    },

    WithdrawPrincipal {
        share_id: u32,
    },

    RegisterNewShare,

    ClaimStakeRewardsBatch,

    ClaimStakeReward {
        share_id: u32,
    },

    ClaimStakePrincipal {
        share_id: u32,
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

    #[returns(ShareInfo)]
    ShareInfo { share_id: u32 },

    #[returns(ShareholderInfo)]
    Shareholder { share_id: u32, addr: String },

    #[returns(ShareInfosResponse)]
    ShareInfos,

    #[returns(HoldersResponse)]
    Shareholders,

    #[returns(TokenConfig)]
    Token,

    #[returns[bool]]
    HaveRole { role: String, account: String },

    #[returns(u128)]
    HeldFunds,
}

#[cw_serde]
pub struct ShareInfosResponse {
    pub share_ids: Vec<u32>,
}

#[cw_serde]
pub struct HoldersResponse {
    pub shareholders: Vec<(Addr, u32)>,
}
