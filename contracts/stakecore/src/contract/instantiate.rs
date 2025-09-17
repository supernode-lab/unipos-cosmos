use crate::msg::InstantiateMsg;
use crate::state::ac::{get_access_control, PROVIDER_ROLE};
use crate::state::ut::TOKEN_CONFIG;
use crate::state::{AssetInfo, Config, ASSET_INFO, CONFIG};
use crate::ContractError;
use access_control::DEFAULT_ADMIN_ROLE;
use cosmwasm_std::{entry_point,DepsMut, Env, MessageInfo, Response, Uint128, Uint64};
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

    if msg.lock_period == Uint64::zero() {
        return Err(ContractError::InvalidInput);
    }

    if msg.installment_num == Uint128::zero() {
        return Err(ContractError::InvalidInput);
    }

    let config = Config {
        lock_period: msg.lock_period,
        cliff_period: msg.cliff_period,
        apy: msg.apy,
        installment_num: msg.installment_num,
        min_stake_amount: msg.min_stake_amount,
        enable_staker_whitelist: msg.enable_staker_whitelist,
        enable_beneficiary_whitelist: msg.enable_beneficiary_whitelist,
    };
    CONFIG.save(deps.storage, &config)?;

    let token_config = TokenConfig {
        is_native: msg.is_native_token,
        token: msg.token,
    };

    TOKEN_CONFIG.save(deps.storage, &token_config)?;
    ASSET_INFO.save(deps.storage, &AssetInfo::default())?;

    let access_control = get_access_control();
    access_control._grant_role(
        deps.storage,
        DEFAULT_ADMIN_ROLE,
        &(deps.api.addr_validate(msg.admin.as_str())?),
    )?;
    access_control.set_role_admin(deps.storage, PROVIDER_ROLE, &PROVIDER_ROLE.to_string())?;
    for provider in msg.providers {
        access_control._grant_role(
            deps.storage,
            PROVIDER_ROLE,
            &(deps.api.addr_validate(&provider)?),
        )?;
    }

    Ok(Response::default())
}
