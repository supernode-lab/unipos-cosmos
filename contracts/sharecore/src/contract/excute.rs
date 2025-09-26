use crate::error::ContractError;
use crate::msg::ExecuteMsg;
use crate::state::ac::get_access_control;
use crate::state::ut::{get_univeral_token, TOKEN_CONFIG};
use crate::state::{
    sub_held_funds, ShareInfo, ShareholderInfo, CONFIG, HELD_FUNDS, PENDING_SUBMSG_PAYLOAD,
    SHAREHOLDER_INFOS, SHARE_INFOS, SHARE_INFO_INDEXES,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    ensure, entry_point, to_json_binary, to_json_string, to_json_vec, DepsMut, Env, MessageInfo,
    QueryRequest, Response, SubMsg, Uint128, Uint64, WasmMsg, WasmQuery,
};
use stakecore::msg::{
    ExecuteMsg as StakecoreExecuteMsg, QueryMsg as StakecoreQueryMsg, StakerIndexesResponse,
};
use stakecore::types::{Config as StakecoreConfig, StakeInfo};
use universal_token::{TokenConfig, UniversalToken};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitStakeCore { stakecore } => {
            execute_init_stakecore(deps, env, info, stakecore)
        }
        ExecuteMsg::AccrueRewards {
            share_id,
            recycled_time,
        } => execute_accrue_rewards(deps, env, info, share_id, recycled_time),
        ExecuteMsg::Recycle { share_id, amount } => {
            execute_recycle(deps, env, info, share_id, amount)
        }
        ExecuteMsg::AddShareholder {
            owner,
            share_id,
            granted_rewards,
            granted_principal,
        } => execute_add_shareholder(
            deps,
            env,
            info,
            owner,
            share_id,
            granted_rewards,
            granted_principal,
        ),

        ExecuteMsg::AddShareholderWithTime {
            owner,
            share_id,
            start_time,
            granted_rewards,
            granted_principal,
        } => execute_add_shareholder_with_time(
            deps,
            env,
            info,
            owner,
            share_id,
            start_time,
            granted_rewards,
            granted_principal,
        ),

        ExecuteMsg::Share {
            share_id,
            new_owner,
            granted_rewards,
            granted_principal,
        } => execute_share(
            deps,
            env,
            info,
            share_id,
            new_owner,
            granted_rewards,
            granted_principal,
        ),

        ExecuteMsg::WithdrawRewards { share_id } => {
            execute_withdraw_rewards(deps, env, info, share_id)
        }
        ExecuteMsg::WithdrawPrincipal { share_id } => {
            execute_withdraw_principal(deps, env, info, share_id)
        }

        ExecuteMsg::RegisterNewShare => execute_register_new_share(deps, env, info),
        ExecuteMsg::Collect {
            is_native_token,
            token,
        } => execute_collect(deps, env, info, is_native_token, token),

        ExecuteMsg::ClaimStakeRewards { share_id } => {
            execute_claim_stake_rewards(deps, env, info, share_id)
        }
        ExecuteMsg::ClaimStakeRewardsBatch => execute_claim_stake_rewards_batch(deps, env, info),

        ExecuteMsg::ClaimStakePrincipal { share_id } => {
            execute_claim_stake_principal(deps, env, info, share_id)
        }
        ExecuteMsg::GrantRole { role, account } => {
            execute_grant_role(deps, env, info, role, account)
        }
    }
}

fn execute_init_stakecore(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stakecore: String,
) -> Result<Response, ContractError> {
    get_access_control().only_admin(deps.storage, &info.sender)?;
    let mut config = CONFIG.load(deps.storage)?;
    if config.stakecore.is_some() {
        return Err(ContractError::StakecoreAlreadyInitialized {});
    }

    let stakecore_addr = deps.api.addr_validate(&stakecore)?;
    let stakecore_token_config: TokenConfig =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: stakecore_addr.to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::Token {})?,
        }))?;

    let token_config = TOKEN_CONFIG.load(deps.storage)?;
    if token_config != stakecore_token_config {
        return Err(ContractError::InvalidParamter {
            key: "stakecore".to_string(),
        });
    }

    config.stakecore = Some(stakecore_addr.clone());
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "init_stakecore")
        .add_attribute("stake_core_addr", stakecore_addr))
}

fn execute_accrue_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    share_id: u32,
    recycled_time: Uint64,
) -> Result<Response, ContractError> {
    get_access_control().only_admin(deps.storage, &info.sender)?;
    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    if recycled_time <= share_info.recycled_time
        || recycled_time > share_info.end_time
        || recycled_time > env.block.time.seconds().into()
    {
        return Err(ContractError::InvalidParamter {
            key: "recycled_time".to_string(),
        });
    }

    let recyclable_rewards = share_info.recyclable_rewards(recycled_time);
    share_info.total_recycled_rewards += recyclable_rewards;
    share_info.recycled_time = recycled_time;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;

    Ok(Response::new()
        .add_attribute("action", "accure_rewards")
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("recycled_time", recycled_time.to_string())
        .add_attribute("recycled_rewards", recyclable_rewards.to_string()))
}

fn execute_recycle(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
    amount: Uint128,
) -> Result<Response, ContractError> {
    get_access_control().only_admin(deps.storage, &info.sender)?;
    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    let available = share_info.total_recycled_rewards - share_info.withdrawn_recycled_rewards;
    ensure!(
        amount <= available,
        ContractError::AmountExceedsWithdrawable
    );

    let withdrawable_rewards = share_info.rewards_balance();
    ensure!(
        amount <= withdrawable_rewards,
        ContractError::AmountExceedsBalance
    );

    share_info.withdrawn_rewards += amount;
    share_info.withdrawn_recycled_rewards += amount;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;
    sub_held_funds(deps.storage, amount)?;

    let msgs = get_univeral_token(deps.storage)?.send_token(&info.sender, amount)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "recycle")
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("amount", amount.to_string()))
}

fn execute_add_shareholder(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    share_id: u32,
    granted_rewards: Uint128,
    granted_principal: Uint128,
) -> Result<Response, ContractError> {
    let share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    _add_shareholder(
        deps,
        env,
        info,
        owner,
        share_id,
        share_info.recycled_time,
        granted_rewards,
        granted_principal,
    )
}

fn execute_add_shareholder_with_time(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    share_id: u32,
    start_time: Uint64,
    granted_rewards: Uint128,
    granted_principal: Uint128,
) -> Result<Response, ContractError> {
    _add_shareholder(
        deps,
        env,
        info,
        owner,
        share_id,
        start_time,
        granted_rewards,
        granted_principal,
    )
}

fn _add_shareholder(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    share_id: u32,
    start_time: Uint64,
    granted_rewards: Uint128,
    granted_principal: Uint128,
) -> Result<Response, ContractError> {
    let access_control = get_access_control();
    access_control.only_admin(deps.storage, &info.sender)?;

    let owner_addr = deps.api.addr_validate(&owner)?;
    let config = CONFIG.load(deps.storage)?;
    if config.enable_shareholder_whitelist {
        access_control.only_shareholder(deps.storage, &owner_addr)?;
    }

    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;

    ensure!(
        start_time >= share_info.recycled_time,
        ContractError::InvalidParamter {
            key: "start_time".to_string()
        }
    );
    ensure!(
        start_time <= share_info.end_time,
        ContractError::InvalidParamter {
            key: "start_time".to_string()
        }
    );
    ensure!(
        start_time != share_info.end_time || granted_rewards == Uint128::zero(),
        ContractError::InvalidParamter {
            key: "start_time".to_string()
        }
    );
    ensure!(
        share_info.granted_principal + granted_principal <= share_info.total_principal,
        ContractError::InvalidParamter {
            key: "granted_principal".to_string()
        }
    );

    let (unrecycled_rewards, need_to_recycle_rewards) = if granted_rewards != Uint128::zero() {
        (
            calc_proportional_rewards(
                start_time,
                share_info.recycled_time,
                share_info.end_time,
                granted_rewards,
            ),
            calc_proportional_rewards(
                start_time,
                share_info.start_time,
                share_info.end_time,
                granted_rewards,
            ),
        )
    } else {
        (Uint128::zero(), Uint128::zero())
    };

    ensure!(
        granted_rewards
            + unrecycled_rewards
            + share_info.granted_rewards
            + share_info.total_recycled_rewards
            <= share_info.total_rewards,
        ContractError::InvalidParamter {
            key: "granted_rewards".to_string()
        }
    );

    share_info.granted_rewards += granted_rewards;
    share_info.granted_principal += granted_principal;
    share_info.total_recycled_rewards += unrecycled_rewards;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;

    let shareholder_info = if let Some(mut shareholder_info) =
        SHAREHOLDER_INFOS.may_load(deps.storage, (&owner_addr, share_id))?
    {
        shareholder_info.granted_rewards += granted_rewards;
        shareholder_info.granted_principal += granted_principal;
        shareholder_info.pre_recycled_rewards += need_to_recycle_rewards;
        shareholder_info
    } else {
        ShareholderInfo {
            pre_recycled_rewards: need_to_recycle_rewards,
            granted_rewards,
            withdrawn_rewards: Uint128::zero(),
            granted_principal: granted_principal,
            withdrawn_principal: Uint128::zero(),
        }
    };

    SHAREHOLDER_INFOS.save(deps.storage, (&owner_addr, share_id), &shareholder_info)?;

    Ok(Response::new()
        .add_attribute("action", "add_shareholder")
        .add_attribute("owner", owner_addr.to_string())
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("start_time", start_time.to_string())
        .add_attribute(
            "need_to_recycle_rewards",
            need_to_recycle_rewards.to_string(),
        )
        .add_attribute("granted_rewards", granted_rewards.to_string())
        .add_attribute("granted_principal", granted_principal.to_string()))
}

fn execute_share(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
    new_owner: String,
    granted_rewards: Uint128,
    granted_principal: Uint128,
) -> Result<Response, ContractError> {
    let new_owner_addr = deps.api.addr_validate(&new_owner)?;
    ensure!(
        granted_rewards + granted_principal > Uint128::zero(),
        ContractError::InvalidParamter {
            key: "granted_rewards&granted_principal".to_string()
        }
    );

    let access_control = get_access_control();
    access_control.only_admin(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    if config.enable_shareholder_whitelist {
        access_control.only_shareholder(deps.storage, &new_owner_addr)?;
    }

    let mut shareholder_info = SHAREHOLDER_INFOS.load(deps.storage, (&info.sender, share_id))?;
    ensure!(
        granted_rewards + shareholder_info.withdrawn_rewards <= shareholder_info.granted_rewards,
        ContractError::InsufficientRewards
    );
    ensure!(
        granted_principal + shareholder_info.withdrawn_principal
            <= shareholder_info.granted_principal,
        ContractError::InsufficientPrincipal
    );

    let pre_recycled_rewards: Uint128 = if shareholder_info.granted_rewards == Uint128::zero() {
        Uint128::zero()
    } else {
        granted_rewards * shareholder_info.pre_recycled_rewards / shareholder_info.granted_rewards
    };

    shareholder_info.granted_rewards -= granted_rewards;
    shareholder_info.granted_principal -= granted_principal;
    shareholder_info.pre_recycled_rewards -= pre_recycled_rewards;
    SHAREHOLDER_INFOS.save(deps.storage, (&info.sender, share_id), &shareholder_info)?;

    let new_shareholder_info = if let Some(mut new_shareholder_info) =
        SHAREHOLDER_INFOS.may_load(deps.storage, (&new_owner_addr, share_id))?
    {
        new_shareholder_info.granted_rewards += granted_rewards;
        new_shareholder_info.granted_principal += granted_principal;
        new_shareholder_info.pre_recycled_rewards += pre_recycled_rewards;
        new_shareholder_info
    } else {
        ShareholderInfo {
            pre_recycled_rewards: pre_recycled_rewards,
            granted_rewards,
            withdrawn_rewards: Uint128::zero(),
            granted_principal: granted_principal,
            withdrawn_principal: Uint128::zero(),
        }
    };

    SHAREHOLDER_INFOS.save(
        deps.storage,
        (&new_owner_addr, share_id),
        &new_shareholder_info,
    )?;

    Ok(Response::new()
        .add_attribute("action", "share")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("new_owner", new_owner)
        .add_attribute("need_to_recycle_rewards", pre_recycled_rewards.to_string())
        .add_attribute("granted_rewards", granted_rewards.to_string())
        .add_attribute("granted_principal", granted_principal.to_string()))
}

fn calc_proportional_rewards(
    cur_t: Uint64,
    start_t: Uint64,
    end_t: Uint64,
    granted_rewards: Uint128,
) -> Uint128 {
    granted_rewards * Uint128::from(cur_t - start_t) / Uint128::from(end_t - cur_t)
}

fn execute_withdraw_rewards(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    let mut shareholder_info = SHAREHOLDER_INFOS.load(deps.storage, (&info.sender, share_id))?;
    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    let mut withdrawable_rewards =
        share_info.calc_shareholder_withdrawable_rewards(&shareholder_info);
    ensure!(
        withdrawable_rewards > Uint128::zero(),
        ContractError::AmountExceedsWithdrawable
    );
    let rewards_balance = share_info.rewards_balance();
    ensure!(
        rewards_balance > Uint128::zero(),
        ContractError::AmountExceedsBalance
    );
    if rewards_balance < withdrawable_rewards {
        withdrawable_rewards = rewards_balance;
    }

    share_info.withdrawn_rewards += withdrawable_rewards;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;

    shareholder_info.withdrawn_rewards += withdrawable_rewards;
    SHAREHOLDER_INFOS.save(deps.storage, (&info.sender, share_id), &shareholder_info)?;
    sub_held_funds(deps.storage, withdrawable_rewards)?;

    let msgs = get_univeral_token(deps.storage)?.send_token(&info.sender, withdrawable_rewards)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_rewards")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("amount", withdrawable_rewards.to_string()))
}

fn execute_withdraw_principal(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    let mut shareholder_info = SHAREHOLDER_INFOS.load(deps.storage, (&info.sender, share_id))?;
    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    let mut withdrawable_principal =
        share_info.calc_shareholder_withdrawable_principal(&shareholder_info);
    ensure!(
        withdrawable_principal > Uint128::zero(),
        ContractError::AmountExceedsWithdrawable
    );
    let principal_balance = share_info.principal_balance();
    ensure!(
        principal_balance > Uint128::zero(),
        ContractError::AmountExceedsBalance
    );
    if principal_balance < withdrawable_principal {
        withdrawable_principal = principal_balance;
    }

    share_info.withdrawn_principal += withdrawable_principal;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;

    shareholder_info.withdrawn_principal += withdrawable_principal;
    SHAREHOLDER_INFOS.save(deps.storage, (&info.sender, share_id), &shareholder_info)?;
    sub_held_funds(deps.storage, withdrawable_principal)?;

    let msgs =
        get_univeral_token(deps.storage)?.send_token(&info.sender, withdrawable_principal)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_principal")
        .add_attribute("owner", info.sender.to_string())
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("amount", withdrawable_principal.to_string()))
}

fn execute_collect(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    is_native_token: bool,
    token: String,
) -> Result<Response, ContractError> {
    get_access_control().only_admin(deps.storage, &info.sender)?;

    if !is_native_token {
        deps.api.addr_validate(token.as_str())?;
    }

    let token_config = TOKEN_CONFIG.load(deps.storage)?;
    let new_token = TokenConfig {
        is_native: is_native_token,
        token: token.clone(),
    };

    let held_token = if token_config == new_token {
        HELD_FUNDS.load(deps.storage)?
    } else {
        Uint128::zero()
    };

    let universal_token = UniversalToken::new(new_token);
    let balance = universal_token.balance(deps.as_ref(), &env)?;
    ensure!(balance > held_token, ContractError::NoExcessTokens);

    let extra_token = balance - held_token;
    let msgs = universal_token.send_token(&info.sender, extra_token)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "collect")
        .add_attribute("is_native_token", is_native_token.to_string())
        .add_attribute("token", token)
        .add_attribute("amount", extra_token))
}

fn execute_register_new_share(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let share_ids: StakerIndexesResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stakecore.as_ref().unwrap().to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::StakerIndexes {
                account: env.contract.address.to_string(),
            })?,
        }))?;

    let stakecore_config: StakecoreConfig =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stakecore.as_ref().unwrap().to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::Config {})?,
        }))?;

    let cliff = stakecore_config.cliff_period;

    let share_ids = share_ids.indexes;
    let new_share_len = share_ids.len();

    let mut share_indexes = SHARE_INFO_INDEXES.load(deps.storage)?;
    let cur_share_len = share_indexes.len();
    if cur_share_len >= new_share_len {
        return Ok(Response::new().add_attribute("action", "register"));
    }

    let mut new_n = new_share_len - cur_share_len;
    if new_n > 8 {
        new_n = 8;
    }

    let share_ids = share_ids[cur_share_len..cur_share_len + new_n].to_vec();
    for share_id in &share_ids {
        let stake_info: StakeInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stakecore.as_ref().unwrap().to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::StakeInfo { index: *share_id })?,
        }))?;

        let start_time = stake_info.start_time + cliff;
        let share_info = ShareInfo {
            start_time: start_time,
            recycled_time: start_time,
            end_time: stake_info.start_time + stakecore_config.lock_period,

            total_rewards: stake_info.total_rewards,
            claimed_rewards: Uint128::zero(),
            withdrawn_rewards: Uint128::zero(),
            granted_rewards: Uint128::zero(),

            total_recycled_rewards: Uint128::zero(),
            withdrawn_recycled_rewards: Uint128::zero(),
            total_principal: stake_info.total_principal,
            claimed_principal: Uint128::zero(),
            withdrawn_principal: Uint128::zero(),
            granted_principal: Uint128::zero(),
        };

        SHARE_INFOS.save(deps.storage, *share_id, &share_info)?;
        share_indexes.push(*share_id);
    }

    SHARE_INFO_INDEXES.save(deps.storage, &share_indexes)?;

    Ok(Response::new()
        .add_attribute("action", "register_new_share")
        .add_attribute("share_ids", to_json_string(&share_ids)?))
}

fn execute_claim_stake_rewards(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    if !SHARE_INFOS.has(deps.storage, share_id) {
        return Err(ContractError::InvalidShareId {});
    }

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::WithdrawRewards { index: share_id };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stakecore.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(
        wasm_msg,
        crate::contract::replay::EXECUTE_CLAIM_STAKE_REWARDS__WITHDRAW_REWARDS,
    );
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&share_id)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_rewards")
        .add_attribute("share_id", share_id.to_string()))
}

fn execute_claim_stake_rewards_batch(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let indexes = SHARE_INFO_INDEXES.load(deps.storage)?;

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::WithdrawRewardsBatch {
        indexes: indexes.clone(),
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stakecore.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(
        wasm_msg,
        crate::contract::replay::EXECUTE_CLAIM_STAKE_REWARDS_BATCH__WITHDRAW_REWARDS_BATCH,
    );
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&indexes)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_rewards_batch"))
}

fn execute_claim_stake_principal(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    if !SHARE_INFOS.has(deps.storage, share_id) {
        return Err(ContractError::InvalidShareId {});
    }

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::WithdrawPrincipal { index: share_id };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stakecore.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(
        wasm_msg,
        crate::contract::replay::EXECUTE_CLAIM_STAKE_PRINCIPAL__WITHDRAW_PRINCIPAL,
    );
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&share_id)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_principal")
        .add_attribute("share_id", share_id.to_string()))
}

fn execute_grant_role(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    role: String,
    account: String,
) -> Result<Response, ContractError> {
    get_access_control().grant_role(
        deps.storage,
        &info.sender,
        &role,
        &deps.api.addr_validate(&account)?,
    )?;

    Ok(Response::new()
        .add_attribute("action", "grant_role")
        .add_attribute("role", role)
        .add_attribute("account", account))
}
