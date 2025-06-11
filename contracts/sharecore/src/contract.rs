use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::query::{HoldersResponse, QueryMsg, ShareInfosResponse};
use crate::state::{
    Config, CONFIG, HOLDER_INFOS, PENDING_SUBMSG_PAYLOAD, SHARE_INFOS, SHARE_INFO_INDEXES,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, to_json_binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Reply, Response,
    SubMsg, Uint128, WasmMsg, WasmQuery,
};
use cosmwasm_std::{from_json, to_json_vec, Binary, Event, Order, StdResult, SubMsgResult};
use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use unipos::sharecore::{HolderInfo, ShareInfo};
use unipos::stakecore::msg::ExecuteMsg as StakecoreExecuteMsg;
use unipos::stakecore::query::{QueryMsg as StakecoreQueryMsg, StakerIndexesResponse};
use unipos::stakecore::state::Config as StakecoreConfig;
use unipos::stakecore::StakeInfo;
use unipos::Ownership;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let stake_core = msg
        .stake_core
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let token = if stake_core.is_some() {
        let stake_core_addr = stake_core.as_ref().unwrap();
        let stake_core_config: StakecoreConfig =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: stake_core_addr.to_string(),
                msg: to_json_binary(&StakecoreQueryMsg::Config)?,
            }))?;
        Some(stake_core_config.token)
    } else {
        None
    };

    let config = Config {
        stake_core,
        token,
        admin: Ownership::new(deps.api.addr_validate(&msg.admin)?),
    };
    CONFIG.save(deps.storage, &config)?;
    SHARE_INFO_INDEXES.save(deps.storage, &vec![])?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::InitStakeCore { stake_core } => {
            execute_init_stake_core(deps, env, info, stake_core)
        }
        ExecuteMsg::AddHolder {
            owner,
            share_id,
            granted_reward,
            granted_principal,
        } => execute_add_holder(
            deps,
            env,
            info,
            owner,
            share_id,
            granted_reward,
            granted_principal,
        ),
        ExecuteMsg::SyncStakeInfo => execute_sync_stake_info(deps, env, info),
        ExecuteMsg::ClaimStakeRewardsBatch => execute_claim_stake_rewards_batch(deps, env, info),
        ExecuteMsg::ClaimStakeReward { share_id } => {
            execute_claim_stake_reward(deps, env, info, share_id)
        }
        ExecuteMsg::ClaimStakePrincipal { share_id } => {
            execute_claim_stake_principal(deps, env, info, share_id)
        }
        ExecuteMsg::ClaimReward { share_id } => execute_claim_reward(deps, env, info, share_id),
        ExecuteMsg::ClaimPrincipal { share_id } => {
            execute_claim_principal(deps, env, info, share_id)
        }
        ExecuteMsg::Collect => execute_collect(deps, env, info),
    }
}

fn execute_init_stake_core(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stake_core: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    if config.stake_core.is_some() {
        return Err(ContractError::StakecoreAlreadyInitialized {});
    }

    let stake_core_addr = deps.api.addr_validate(&stake_core)?;
    let stake_core_config: StakecoreConfig =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: stake_core_addr.to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::Config)?,
        }))?;

    config.stake_core = Some(stake_core_addr.clone());
    config.token = Some(stake_core_config.token);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "init_stake_core")
        .add_attribute("stake_core_addr", stake_core_addr))
}

fn execute_add_holder(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    share_id: u32,
    granted_reward: Uint128,
    granted_principal: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let owner_addr = deps.api.addr_validate(&owner)?;
    if HOLDER_INFOS.has(deps.storage, (&owner_addr, share_id)) {
        return Err(ContractError::HolderAlreadyRegistered {});
    }

    let mut share_info = SHARE_INFOS.load(deps.storage, share_id)?;
    if share_info.total_reward < share_info.granted_reward + granted_reward {
        return Err(ContractError::InsufficientReward {});
    }

    if share_info.principal < share_info.granted_principal + granted_principal {
        return Err(ContractError::InsufficientPrincipal {});
    }

    share_info.granted_reward += granted_reward;
    share_info.granted_principal += granted_principal;
    SHARE_INFOS.save(deps.storage, share_id, &share_info)?;

    HOLDER_INFOS.save(
        deps.storage,
        (&owner_addr, share_id),
        &HolderInfo {
            granted_reward: granted_reward,
            claimed_reward: Uint128::zero(),
            granted_principal: granted_principal,
            claimed_principal: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_attribute("action", "add_holder"))
}

fn execute_sync_stake_info(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let share_ids: StakerIndexesResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stake_core.as_ref().unwrap().to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::StakerIndexes {
                staker: env.contract.address.to_string(),
            })?,
        }))?;

    let mut share_indexes = SHARE_INFO_INDEXES.load(deps.storage)?;
    if share_indexes.len() >= share_ids.indexes.len() {
        return Ok(Response::new().add_attribute("action", "register"));
    }

    let mut share_infos = Vec::new();
    for share_id in share_ids.indexes.iter().skip(share_indexes.len()) {
        let stake_info: StakeInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.stake_core.as_ref().unwrap().to_string(),
            msg: to_json_binary(&StakecoreQueryMsg::StakeInfo { index: *share_id })?,
        }))?;

        share_infos.push((
            *share_id,
            ShareInfo {
                total_reward: stake_info.total_reward,
                claimed_reward: Uint128::zero(),
                granted_reward: Uint128::zero(),
                principal: stake_info.amount,
                claimed_principal: Uint128::zero(),
                granted_principal: Uint128::zero(),
            },
        ));
        share_indexes.push(*share_id);
    }

    for (share_id, share_info) in share_infos {
        SHARE_INFOS.save(deps.storage, share_id, &share_info)?;
    }

    SHARE_INFO_INDEXES.save(deps.storage, &share_indexes)?;

    Ok(Response::new().add_attribute("action", "sync_stake_info"))
}

fn execute_claim_stake_rewards_batch(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    let indexes = SHARE_INFO_INDEXES.load(deps.storage)?;

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::ClaimRewardsBatch {
        indexes: indexes.clone(),
    };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stake_core.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(
        wasm_msg,
        EXECUTE_CLAIM_STAKE_REWARDS_BATCH__CLAIM_REWARDS_BATCH,
    );
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&indexes)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_rewards_batch"))
}

fn execute_claim_stake_reward(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    if !SHARE_INFOS.has(deps.storage, share_id) {
        return Err(ContractError::ShareInfoNotFound {});
    }

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::ClaimReward { index: share_id };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stake_core.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(wasm_msg, EXECUTE_CLAIM_STAKE_REWARD__CLAIM_REWARD);
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&share_id)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_reward")
        .add_attribute("share_id", share_id.to_string()))
}

fn execute_claim_stake_principal(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    if !SHARE_INFOS.has(deps.storage, share_id) {
        return Err(ContractError::ShareInfoNotFound {});
    }

    let config = CONFIG.load(deps.storage)?;
    let msg = StakecoreExecuteMsg::Unstake { index: share_id };
    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.stake_core.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
        funds: vec![],
    };

    let sub_msg = SubMsg::reply_on_success(wasm_msg, EXECUTE_CLAIM_STAKE_PRINCIPAL__UNSTAKE);
    PENDING_SUBMSG_PAYLOAD.save(deps.storage, &to_json_vec(&share_id)?)?;

    Ok(Response::default()
        .add_submessage(sub_msg)
        .add_attribute("action", "claim_stake_principal")
        .add_attribute("share_id", share_id.to_string()))
}

fn execute_claim_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    let mut holder_info = HOLDER_INFOS.load(deps.storage, (&info.sender, share_id))?;
    let share_info = SHARE_INFOS
        .load(deps.storage, share_id)
        .map_err(|_| ContractError::ShareInfoNotFound {})?;

    let claimable_total_reward = share_info.calc_holder_reward(holder_info.granted_reward);
    if claimable_total_reward <= holder_info.claimed_reward {
        return Err(ContractError::NoReward {});
    }

    let claimable_reward = claimable_total_reward - holder_info.claimed_reward;
    holder_info.claimed_reward = claimable_total_reward;
    HOLDER_INFOS.save(deps.storage, (&info.sender, share_id), &holder_info)?;

    let config = CONFIG.load(deps.storage)?;
    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: claimable_reward,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.unwrap().to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_reward")
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("amount", claimable_reward))
}

fn execute_claim_principal(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    share_id: u32,
) -> Result<Response, ContractError> {
    let mut holder_info = HOLDER_INFOS.load(deps.storage, (&info.sender, share_id))?;
    let share_info = SHARE_INFOS
        .load(deps.storage, share_id)
        .map_err(|_| ContractError::ShareInfoNotFound {})?;

    let claimable_total_principal = share_info.calc_holder_principal(holder_info.granted_principal);
    if claimable_total_principal <= holder_info.claimed_principal {
        return Err(ContractError::NoPrincipal {});
    }

    let claimable_principal = claimable_total_principal - holder_info.claimed_principal;
    holder_info.claimed_principal = claimable_principal;
    HOLDER_INFOS.save(deps.storage, (&info.sender, share_id), &holder_info)?;

    let config = CONFIG.load(deps.storage)?;
    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: claimable_principal,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.unwrap().to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_principal")
        .add_attribute("share_id", share_id.to_string())
        .add_attribute("amount", claimable_principal))
}

fn execute_collect(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized {});
    }

    let msg = Cw20QueryMsg::Balance {
        address: env.contract.address.to_string(),
    };

    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.token.as_ref().unwrap().to_string(),
        msg: to_json_binary(&msg)?,
    }))?;

    let mut total_balance = Uint128::zero();

    for res in SHARE_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let (_, share_info) = res?;
        total_balance += share_info.claimed_reward + share_info.claimed_principal;
    }

    for res in HOLDER_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let (_, holder_info) = res?;
        total_balance -= holder_info.claimed_reward + holder_info.claimed_principal;
    }

    if balance.balance <= total_balance {
        return Ok(Response::default()
            .add_attribute("action", "collect")
            .add_attribute("amount", Uint128::zero()));
    }

    let collectable = balance.balance - total_balance;
    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: collectable,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.as_ref().unwrap().to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "collect")
        .add_attribute("amount", collectable))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::ShareInfo { share_id } => to_json_binary(&query_share_info(deps, share_id)?),
        QueryMsg::HolderInfo { share_id, addr } => {
            to_json_binary(&query_holder_info(deps, share_id, addr)?)
        }
        QueryMsg::ShareInfos => to_json_binary(&query_share_infos(deps)?),
        QueryMsg::Holders => to_json_binary(&query_holders(deps)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_share_info(deps: Deps, share_id: u32) -> StdResult<ShareInfo> {
    SHARE_INFOS.load(deps.storage, share_id)
}

fn query_holder_info(deps: Deps, share_id: u32, addr: String) -> StdResult<HolderInfo> {
    let holder_addr = deps.api.addr_validate(&addr)?;
    HOLDER_INFOS.load(deps.storage, (&holder_addr, share_id))
}

fn query_share_infos(deps: Deps) -> StdResult<ShareInfosResponse> {
    let mut share_ids = Vec::new();
    for res in SHARE_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let (share_id, _) = res?;
        share_ids.push(share_id);
    }

    Ok(ShareInfosResponse { share_ids })
}

fn query_holders(deps: Deps) -> StdResult<HoldersResponse> {
    let mut holders = Vec::new();
    for res in HOLDER_INFOS.range(deps.storage, None, None, Order::Ascending) {
        let ((addr, share_id), _) = res?;
        holders.push((addr, share_id));
    }

    Ok(HoldersResponse { holders })
}

const EXECUTE_CLAIM_STAKE_REWARD__CLAIM_REWARD: u64 = 1;
const EXECUTE_CLAIM_STAKE_REWARDS_BATCH__CLAIM_REWARDS_BATCH: u64 = 2;
const EXECUTE_CLAIM_STAKE_PRINCIPAL__UNSTAKE: u64 = 3;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        SubMsgResult::Err(err) => return Err(ContractError::SubMsgFailed { msg: err }),
        _ => {}
    };

    match msg.id {
        EXECUTE_CLAIM_STAKE_REWARD__CLAIM_REWARD => reply_claim_stake_reward(deps, env, msg),
        EXECUTE_CLAIM_STAKE_REWARDS_BATCH__CLAIM_REWARDS_BATCH => {
            reply_claim_stake_rewards_batch(deps, env, msg)
        }
        EXECUTE_CLAIM_STAKE_PRINCIPAL__UNSTAKE => reply_claim_stake_principal(deps, env, msg),

        id => Err(ContractError::UnknownReplyId { id }),
    }
}

fn reply_claim_stake_reward(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amount = get_value_from_event(&resp.events, "claim_reward", "amount").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in claim_reward"),
        },
    )?;

    let payload = PENDING_SUBMSG_PAYLOAD.load(deps.storage)?;
    let amount: Uint128 = amount.parse()?;
    let share_id = from_json(payload)?;
    SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
        let mut share_info = old.ok_or(ContractError::ShareInfoNotFound {})?;
        share_info.claimed_reward += amount;
        Ok(share_info)
    })?;

    Ok(Response::default())
}

fn reply_claim_stake_rewards_batch(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amounts = get_value_from_event(&resp.events, "claim_rewards_batch", "amounts").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in claim_rewards_batch"),
        },
    )?;

    let amounts: Vec<Uint128> = from_json(amounts)?;
    let payload = PENDING_SUBMSG_PAYLOAD.load(deps.storage)?;
    let share_ids: Vec<u32> = from_json(payload)?;
    if amounts.len() != share_ids.len() {
        return Err(ContractError::InnerError {
            msg: format!(
                "share_ids and amounts length do not match: expected: {}, actual: {}",
                share_ids.len(),
                amounts.len()
            ),
        });
    }

    for (share_id, amount) in share_ids.into_iter().zip(amounts.into_iter()) {
        SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
            let mut share_info = old.ok_or(ContractError::ShareInfoNotFound {})?;
            share_info.claimed_reward += amount;
            Ok(share_info)
        })?;
    }

    Ok(Response::default())
}

fn reply_claim_stake_principal(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amount = get_value_from_event(&resp.events, "unstake", "amount").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in unstake"),
        },
    )?;

    let amount: Uint128 = amount.parse()?;
    let payload = PENDING_SUBMSG_PAYLOAD.load(deps.storage)?;
    let share_id = from_json(payload)?;
    SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
        let mut share_info = old.ok_or(ContractError::ShareInfoNotFound {})?;
        share_info.claimed_principal += amount;
        Ok(share_info)
    })?;

    Ok(Response::default())
}

fn get_value_from_event(events: &[Event], action: &str, key: &str) -> Option<String> {
    let mut found = false;
    let mut result = None;

    for e in events {
        for attr in &e.attributes {
            if attr.key == "action" && attr.value == action {
                found = true
            } else if attr.key == key {
                result = Some(attr.value.clone());
            }
        }

        if found {
            return result;
        }
    }

    None
}


#[cfg(test)]
mod tests {}
