use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::ac::get_access_control;
use crate::state::ut::TOKEN_CONFIG;
use crate::state::{Config, CONFIG, HELD_FUNDS, SHARE_INFO_INDEXES};
use access_control::DEFAULT_ADMIN_ROLE;

#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, DepsMut, Env, MessageInfo, QueryRequest, Response, Uint128,
    WasmQuery,
};
use stakecore::msg::QueryMsg as StakecoreQueryMsg;

use universal_token::TokenConfig;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    //check arguments
    if !msg.is_native_token {
        deps.api.addr_validate(msg.token.as_str())?;
    }

    let admin_addr = deps.api.addr_validate(&msg.admin)?;

    let stakecore = if msg.stakecore.is_some() {
        let stakecore_addr = deps.api.addr_validate(msg.stakecore.unwrap().as_str())?;
        let stakecore_token_config: TokenConfig =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: stakecore_addr.to_string(),
                msg: to_json_binary(&StakecoreQueryMsg::Token {})?,
            }))?;

        if msg.is_native_token != stakecore_token_config.is_native
            || msg.token != stakecore_token_config.token
        {
            return Err(ContractError::InvalidParamter {
                key: "token".to_string(),
            });
        }
        Some(stakecore_addr)
    } else {
        None
    };

    let config = Config {
        stakecore,
        enable_shareholder_whitelist: msg.enable_shareholder_whitelist,
    };
    CONFIG.save(deps.storage, &config)?;

    HELD_FUNDS.save(deps.storage, &Uint128::zero())?;

    let token_config: TokenConfig = TokenConfig {
        is_native: msg.is_native_token,
        token: msg.token,
    };
    TOKEN_CONFIG.save(deps.storage, &token_config)?;

    SHARE_INFO_INDEXES.save(deps.storage, &vec![])?;

    get_access_control()._grant_role(deps.storage, DEFAULT_ADMIN_ROLE, &admin_addr)?;

    Ok(Response::default())
}
