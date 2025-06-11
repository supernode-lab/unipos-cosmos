use bh_storage::Vector;
use cosmwasm_std::{Addr};
use cw_storage_plus::{ Item, Map};
pub use unipos::stakecore::state::*;


pub const CONFIG: Item<Config> = Item::new("config");

pub const BENEFICIARY_INFO: Item<BeneficiaryInfo> = Item::new("beneficiary_info");

pub const ASSET_INFO: Item<AssetInfo> = Item::new("asset_info");

pub const STAKE_INFOS: Vector<StakeInfo> = Vector::new("stake_infos");

pub const STAKER_INDEXES: Map<&Addr, Vec<u32>> = Map::new("staker_indexes");
