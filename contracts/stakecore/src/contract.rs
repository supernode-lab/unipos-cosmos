use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg};
use crate::state::{
    AssetInfo, BeneficiaryInfo, Config, StakeInfo, ASSET_INFO, BENEFICIARY_INFO, CONFIG,
    STAKER_INDEXES, STAKE_INFOS,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_json_binary, to_json_string, Binary, Deps, DepsMut, Env, MessageInfo, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery};
use unipos::stakecore::query::QueryMsg;
use unipos::Ownership;

use cw20::{BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg};
use unipos::stakecore::StakerIndexesResponse;
/*
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:unipos-cosm";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
*/

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;

    let config = Config {
        token: deps.api.addr_validate(msg.token.as_str())?,
        lock_period: msg.lock_period,
        staker_reward_share: msg.staker_shares,
        apy: msg.apy,
        min_stake_amount: msg.min_stake_amount,
        installment_num: msg.installment_num,
        provider: Ownership::new(deps.api.addr_validate(msg.provider.as_str())?),
        admin: Ownership::new(deps.api.addr_validate(msg.admin.as_str())?),
    };
    CONFIG.save(deps.storage, &config)?;
    BENEFICIARY_INFO.save(deps.storage, &BeneficiaryInfo::default())?;
    ASSET_INFO.save(deps.storage, &AssetInfo::default())?;

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
        ExecuteMsg::TransferProviderOwnership { new_provider } => {
            execute_transfer_provider_ownership(deps, env, info, new_provider)
        }
        ExecuteMsg::AcceptProviderOwnership => execute_accept_provider_ownership(deps, env, info),
        ExecuteMsg::TransferAdminOwnership { new_admin } => {
            execute_transfer_admin_ownership(deps, env, info, new_admin)
        }
        ExecuteMsg::AcceptAdminOwnership => execute_accept_admin_ownership(deps, env, info),
        ExecuteMsg::InitBeneficiary { bf } => execute_init_beneficiary(deps, env, info, bf),
        ExecuteMsg::DepositSecurity { amount } => execute_deposit_security(deps, env, info, amount),
        ExecuteMsg::WithdrawSecurity { amount } => {
            execute_withdraw_security(deps, env, info, amount)
        }
        ExecuteMsg::Stake { owner, amount } => execute_stake(deps, env, info, owner, amount),
        ExecuteMsg::Unstake { index } => execute_unstake(deps, env, info, index),
        ExecuteMsg::ClaimReward { index } => execute_claim_reward(deps, env, info, index),
        ExecuteMsg::ClaimRewardsBatch { indexes } => {
            execute_claim_rewards_batch(deps, env, info, indexes)
        }
        ExecuteMsg::ClaimBeneficiaryReward => execute_claim_beneficiary_reward(deps, env, info),
        ExecuteMsg::Collect => execute_collect(deps, env, info),
    }
}

fn execute_transfer_provider_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_provider: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if !config.provider.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    config.provider.pending_owner = Some(deps.api.addr_validate(&new_provider)?);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "transfer_provider_ownership")
        .add_attribute("pending_provider", &new_provider))
}

fn execute_accept_provider_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if !config.provider.accept_ownership(&info.sender) {
        return Err(ContractError::Unauthorized);
    }
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "accept_provider_ownership")
        .add_attribute("new_provider", info.sender))
}
fn execute_transfer_admin_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_admin: String,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    config.admin.pending_owner = Some(deps.api.addr_validate(&new_admin)?);
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "transfer_admin_ownership")
        .add_attribute("pending_admin", &new_admin))
}

fn execute_accept_admin_ownership(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    if !config.admin.accept_ownership(&info.sender) {
        return Err(ContractError::Unauthorized);
    }
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default()
        .add_attribute("action", "accept_admin_ownership")
        .add_attribute("new_admin", info.sender))
}

fn execute_init_beneficiary(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    beneficiary: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    let mut beneficiary_info = BENEFICIARY_INFO.load(deps.storage)?;
    if beneficiary_info.beneficiary.is_some() {
        return Err(ContractError::Unauthorized);
    }

    beneficiary_info.beneficiary = Some(deps.api.addr_validate(&beneficiary)?);
    BENEFICIARY_INFO.save(deps.storage, &beneficiary_info)?;

    Ok(Response::default()
        .add_attribute("action", "init_beneficiary")
        .add_attribute("beneficiary", &beneficiary))
}

fn execute_deposit_security(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.provider.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;
    asset_info.total_security_deposit += amount;
    ASSET_INFO.save(deps.storage, &asset_info)?;

    let cw20_transfer_from_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.to_string(),
        recipient: env.contract.address.to_string(),
        amount,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_from_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "deposit_security")
        .add_attribute("amount", amount)
        .add_attribute("total_security_deposit", asset_info.total_security_deposit))
}

fn execute_withdraw_security(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.provider.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;
    let required_security = config.calc_security_deposit_by_collateral(asset_info.total_collateral);
    let remaining_security = asset_info.total_security_deposit - required_security;
    if remaining_security < amount {
        return Err(ContractError::InsufficientFunds);
    }

    asset_info.total_security_deposit -= amount;
    ASSET_INFO.save(deps.storage, &asset_info)?;

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "withdraw_security")
        .add_attribute("amount", amount)
        .add_attribute("total_security_deposit", asset_info.total_security_deposit))
}

fn execute_stake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    owner: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let owner_addr = deps.api.addr_validate(&owner)?;

    let config = CONFIG.load(deps.storage)?;
    if amount < config.min_stake_amount {
        return Err(ContractError::InsufficientStakeAmount);
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;
    let required_security =
        config.calc_security_deposit_by_collateral(asset_info.total_collateral + amount);
    if required_security > asset_info.total_security_deposit {
        return Err(ContractError::InsufficientFunds);
    }

    asset_info.total_collateral += amount;
    ASSET_INFO.save(deps.storage, &asset_info)?;
    let stake_info = StakeInfo {
        owner: owner_addr.clone(),
        amount: amount,
        start_at: env.block.time.seconds().into(),
        claimed_reward: Uint128::zero(),
        total_reward: config.calc_staker_reward(amount),
        unstaked: false,
    };
    let index = STAKE_INFOS.len(deps.storage)?;
    STAKER_INDEXES.update(deps.storage, &owner_addr, |indexes| -> StdResult<_> {
        Ok(indexes.map_or_else(
            || vec![index],
            |mut i_vec| {
                i_vec.push(index);
                i_vec
            },
        ))
    })?;

    STAKE_INFOS.push(deps.storage, &stake_info)?;

    let cw20_transfer_from_msg = Cw20ExecuteMsg::TransferFrom {
        owner: info.sender.to_string(),
        recipient: env.contract.address.to_string(),
        amount,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_from_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "stake")
        .add_attribute("amount", amount)
        .add_attribute("start_time", stake_info.start_at))
}

fn execute_unstake(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let mut stake_info = STAKE_INFOS.get(deps.storage, index)?;
    if stake_info.owner != info.sender {
        return Err(ContractError::Unauthorized);
    }

    let config = CONFIG.load(deps.storage)?;
    if stake_info.start_at + config.lock_period > env.block.time.seconds().into() {
        return Err(ContractError::LockPeriodNotEnded);
    }

    if stake_info.unstaked {
        return Err(ContractError::AlreadyUnstaked);
    }

    stake_info.unstaked = true;
    STAKE_INFOS.set(deps.storage, index, &stake_info)?;

    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.unstaked_collateral += stake_info.amount;
        Ok(info)
    })?;

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: stake_info.owner.to_string(),
        amount: stake_info.amount,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "unstake")
        .add_attribute("amount", stake_info.amount)
        .add_attribute("index", index.to_string()))
}

fn execute_claim_reward(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let mut stake_info = STAKE_INFOS.get(deps.storage, index)?;
    if stake_info.owner != info.sender {
        return Err(ContractError::Unauthorized);
    }

    let config = CONFIG.load(deps.storage)?;
    let claimable_reward = config.calc_claimable_reward(&env, &stake_info);
    if claimable_reward == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "claim_reward")
            .add_attribute("staker", info.sender)
            .add_attribute("amount", claimable_reward)
            .add_attribute("index", index.to_string()));
    }

    stake_info.claimed_reward += claimable_reward;
    STAKE_INFOS.set(deps.storage, index, &stake_info)?;
    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_claimed_reward += claimable_reward;
        Ok(info)
    })?;

    BENEFICIARY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_reward += config.calc_beneficiary_reward_by_staker_reward(claimable_reward);
        Ok(info)
    })?;

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: claimable_reward,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_reward")
        .add_attribute("staker", info.sender)
        .add_attribute("amount", claimable_reward)
        .add_attribute("index", index.to_string()))
}

fn execute_claim_rewards_batch(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    indexes: Vec<u32>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let mut amounts = Vec::new();
    let mut total_claimable_reward = Uint128::zero();
    for index in &indexes {
        let mut stake_info = STAKE_INFOS.get(deps.storage, *index)?;
        if stake_info.owner != info.sender {
            return Err(ContractError::Unauthorized);
        }

        let claimable_reward = config.calc_claimable_reward(&env, &stake_info);
        amounts.push(claimable_reward);

        if claimable_reward == Uint128::zero() {
            continue;
        }

        stake_info.claimed_reward += claimable_reward;
        STAKE_INFOS.set(deps.storage, *index, &stake_info)?;
        total_claimable_reward += claimable_reward;
    }

    if total_claimable_reward == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "claim_rewards_batch")
            .add_attribute("staker", info.sender)
            .add_attribute("amount", total_claimable_reward)
            .add_attribute("amounts", to_json_string(&amounts)?)
            .add_attribute("indexes", to_json_string(&indexes)?));
    }

    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_claimed_reward += total_claimable_reward;
        Ok(info)
    })?;

    BENEFICIARY_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_reward +=
            config.calc_beneficiary_reward_by_staker_reward(total_claimable_reward);
        Ok(info)
    })?;

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: total_claimable_reward,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_rewards_batch")
        .add_attribute("staker", info.sender)
        .add_attribute("amount", total_claimable_reward)
        .add_attribute("amounts", to_json_string(&amounts)?)
        .add_attribute("indexes", to_json_string(&indexes)?))
}

fn execute_claim_beneficiary_reward(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut beneficiary_info = BENEFICIARY_INFO.load(deps.storage)?;
    if beneficiary_info
        .beneficiary
        .as_ref()
        .ok_or(ContractError::Unauthorized)?
        != info.sender
    {
        return Err(ContractError::Unauthorized {});
    }

    let claimable_reward = beneficiary_info.total_reward - beneficiary_info.claimed_reward;
    if claimable_reward == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "claim_beneficiary_reward")
            .add_attribute("beneficiary", info.sender)
            .add_attribute("amount", claimable_reward));
    }

    beneficiary_info.claimed_reward = beneficiary_info.total_reward;
    BENEFICIARY_INFO.save(deps.storage, &beneficiary_info)?;

    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_claimed_reward += claimable_reward;
        Ok(info)
    })?;

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: info.sender.to_string(),
        amount: claimable_reward,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: CONFIG.load(deps.storage)?.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "claim_beneficiary_reward")
        .add_attribute("beneficiary", info.sender)
        .add_attribute("amount", claimable_reward))
}

fn execute_collect(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if !config.admin.validate(&info.sender) {
        return Err(ContractError::Unauthorized);
    }

    let msg = Cw20QueryMsg::Balance {
        address: env.contract.address.to_string(),
    };

    let balance: BalanceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&msg)?,
    }))?;

    let asset_info = ASSET_INFO.load(deps.storage)?;
    if asset_info.total_security_deposit + asset_info.total_collateral
        >= balance.balance + asset_info.unstaked_collateral + asset_info.total_claimed_reward
    {
        return Err(ContractError::NoLockedFunds {});
    }

    let extra_fund = balance.balance
        - (asset_info.total_collateral + asset_info.total_security_deposit
            - asset_info.unstaked_collateral
            - asset_info.total_claimed_reward);

    let cw20_transfer_msg = Cw20ExecuteMsg::Transfer {
        recipient: config.provider.owner.to_string(),
        amount: extra_fund,
    };

    let wasm_msg = WasmMsg::Execute {
        contract_addr: config.token.to_string(),
        msg: to_json_binary(&cw20_transfer_msg)?,
        funds: vec![],
    };

    Ok(Response::default()
        .add_message(wasm_msg)
        .add_attribute("action", "collect")
        .add_attribute("extra_fund", extra_fund))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config => to_json_binary(&query_config(deps)?),
        QueryMsg::BeneficiaryInfo => to_json_binary(&query_beneficiary_info(deps)?),
        QueryMsg::AssetInfo => to_json_binary(&query_asset_info(deps)?),
        QueryMsg::StakeInfo { index } => to_json_binary(&query_stake_info(deps, index)?),
        QueryMsg::StakerIndexes { staker } => to_json_binary(&query_staker_indexes(deps, staker)?),
    }
}

fn query_config(deps: Deps) -> StdResult<Config> {
    CONFIG.load(deps.storage)
}

fn query_beneficiary_info(deps: Deps) -> StdResult<BeneficiaryInfo> {
    BENEFICIARY_INFO.load(deps.storage)
}

fn query_asset_info(deps: Deps) -> StdResult<AssetInfo> {
    ASSET_INFO.load(deps.storage)
}

fn query_stake_info(deps: Deps, index: u32) -> StdResult<StakeInfo> {
    STAKE_INFOS.get(deps.storage, index)
}

fn query_staker_indexes(deps: Deps, staker: String) -> StdResult<StakerIndexesResponse> {
    STAKER_INDEXES
        .load(deps.storage, &deps.api.addr_validate(&staker)?)
        .map(|indexes| StakerIndexesResponse { indexes })
}


#[cfg(test)]
mod tests {}
