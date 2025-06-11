use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
pub use unipos::sharecore::state::*;

pub const CONFIG: Item<Config> = Item::new("config");

pub const SHARE_INFOS: Map<u32, ShareInfo> = Map::new("share_infos");

pub const SHARE_INFO_INDEXES: Item<Vec<u32>> = Item::new("share_info_indexes");

pub const HOLDER_INFOS: Map<(&Addr, u32), HolderInfo> = Map::new("holder_infos");

pub const PENDING_SUBMSG_PAYLOAD: Item<Vec<u8>> = Item::new("pending_submsg_payload");
