#![allow(unused_imports)]
use crate::stakecore::state::{AssetInfo, BeneficiaryInfo, Config, StakeInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config,
    #[returns(BeneficiaryInfo)]
    BeneficiaryInfo,

    #[returns(AssetInfo)]
    AssetInfo,

    #[returns(StakeInfo)]
    StakeInfo { index: u32 },

    #[returns(StakerIndexesResponse)]
    StakerIndexes { staker: String },
}

#[cw_serde]
pub struct StakerIndexesResponse {
    pub indexes: Vec<u32>,
}
