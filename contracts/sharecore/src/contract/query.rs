use crate::msg::{HoldersResponse, QueryMsg, ShareInfosResponse};
use crate::state::ac::get_access_control;
use crate::state::ut::get_univeral_token;
use crate::state::{
    Config, ShareInfo, ShareholderInfo, CONFIG, HELD_FUNDS, SHAREHOLDER_INFOS, SHARE_INFOS,
};
use cosmwasm_std::{entry_point, to_json_binary, Binary, Deps, Env, Order, StdResult};
use universal_token::TokenConfig;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config => to_json_binary(&query_config(deps)?),
        QueryMsg::ShareInfo { share_id } => to_json_binary(&query_share_info(deps, share_id)?),
        QueryMsg::ShareInfos => to_json_binary(&query_share_infos(deps)?),
        QueryMsg::Shareholder { share_id, addr } => {
            to_json_binary(&query_shareholder(deps, share_id, addr)?)
        }
        QueryMsg::Shareholders => to_json_binary(&query_shareholders(deps)?),
        QueryMsg::Token => to_json_binary(&query_token(deps)?),
        QueryMsg::HaveRole { role, account } => {
            to_json_binary(&query_have_role(deps, role, account)?)
        }
        QueryMsg::HeldFunds => to_json_binary(&query_held_funds(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_token(deps: Deps) -> StdResult<TokenConfig> {
    Ok(get_univeral_token(deps.storage)?.config)
}

fn query_share_info(deps: Deps, share_id: u32) -> StdResult<ShareInfo> {
    SHARE_INFOS.load(deps.storage, share_id)
}

fn query_shareholder(deps: Deps, share_id: u32, addr: String) -> StdResult<ShareholderInfo> {
    let holder_addr = deps.api.addr_validate(&addr)?;
    SHAREHOLDER_INFOS.load(deps.storage, (&holder_addr, share_id))
}

fn query_share_infos(deps: Deps) -> StdResult<ShareInfosResponse> {
    let mut share_ids = Vec::new();
    for res in SHARE_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let (share_id, _) = res?;
        share_ids.push(share_id);
    }

    Ok(ShareInfosResponse { share_ids })
}

fn query_shareholders(deps: Deps) -> StdResult<HoldersResponse> {
    let mut shareholders = Vec::new();
    for res in SHAREHOLDER_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let ((addr, share_id), _) = res?;
        shareholders.push((addr, share_id));
    }

    Ok(HoldersResponse {
        shareholders: shareholders,
    })
}

fn query_have_role(deps: Deps, role: String, account: String) -> StdResult<bool> {
    let account = deps.api.addr_validate(account.as_str())?;
    Ok(get_access_control().has_role(deps.storage, &role, &account))
}

fn query_held_funds(deps: Deps) -> StdResult<u128> {
    Ok(HELD_FUNDS.load(deps.storage)?.u128())
}
