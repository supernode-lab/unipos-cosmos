use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub stake_core: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    InitStakeCore {
        stake_core: String,
    },
    AddHolder {
        owner: String,
        share_id: u32,
        granted_reward: Uint128,
        granted_principal: Uint128,
    },
    SyncStakeInfo,
    ClaimStakeRewardsBatch,
    ClaimStakeReward {
        share_id: u32,
    },
    ClaimStakePrincipal {
        share_id: u32,
    },
    ClaimReward {
        share_id: u32,
    },
    ClaimPrincipal {
        share_id: u32,
    },
    Collect,
}

