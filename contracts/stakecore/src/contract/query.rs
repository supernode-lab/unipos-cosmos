use crate::msg::{QueryMsg, StakerIndexesResponse};
use crate::state::ac::get_access_control;
use crate::state::ut::get_univeral_token;
use crate::state::{
    AssetInfo, Config, StakeInfo, ASSET_INFO, CONFIG, STAKER_INDEXES, STAKE_RECORDS,
};
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, Env, StdResult};
use universal_token::TokenConfig;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config=> to_json_binary(&query_config(deps)?),
        QueryMsg::AssetInfo => to_json_binary(&query_asset_info(deps)?),
        QueryMsg::StakeInfo { index } => to_json_binary(&query_stake_info(deps, index)?),
        QueryMsg::StakerIndexes { staker } => to_json_binary(&query_staker_indexes(deps, staker)?),
        QueryMsg::Token => to_json_binary(&query_token(deps)?),
        QueryMsg::HaveRole { role, account } => {
            to_json_binary(&query_have_role(deps, role, account)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_asset_info(deps: Deps) -> StdResult<AssetInfo> {
    ASSET_INFO.load(deps.storage)
}

fn query_stake_info(deps: Deps, index: u32) -> StdResult<StakeInfo> {
    STAKE_RECORDS.get(deps.storage, index)
}

fn query_staker_indexes(deps: Deps, staker: String) -> StdResult<StakerIndexesResponse> {
    STAKER_INDEXES
        .load(deps.storage, &deps.api.addr_validate(&staker)?)
        .map(|indexes| StakerIndexesResponse { indexes })
}

fn query_token(deps: Deps) -> StdResult<TokenConfig> {
    Ok(get_univeral_token(deps.storage)?.config)
}

fn query_have_role(deps: Deps, role: String, account: String) -> StdResult<bool> {
    let account = deps.api.addr_validate(account.as_str())?;
    Ok(get_access_control().has_role(deps.storage, &role, &account))
}
