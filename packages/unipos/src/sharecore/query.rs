#![allow(unused_imports)]
use crate::sharecore::state::*;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config,

    #[returns(ShareInfo)]
    ShareInfo {
        share_id: u32,
    },

    #[returns(HolderInfo)]
    HolderInfo {
        share_id: u32,
        addr: String,
    },

    #[returns(ShareInfosResponse)]
    ShareInfos,

    #[returns(HoldersResponse)]
    Holders,
}

#[cw_serde]
pub struct ShareInfosResponse {
    pub share_ids: Vec<u32>,
}

#[cw_serde]
pub struct HoldersResponse {
    pub holders: Vec<(Addr, u32)>,
}
