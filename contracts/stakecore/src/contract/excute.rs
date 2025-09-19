use crate::msg::ExecuteMsg;
use crate::state::ac::get_access_control;
use crate::state::ut::{get_univeral_token, TOKEN_CONFIG};
use crate::state::{StakeInfo, ASSET_INFO, CONFIG, STAKER_INDEXES, STAKE_RECORDS};
use crate::ContractError;
use cosmwasm_std::{
    ensure, entry_point, to_json_string, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use universal_token::{TokenConfig, UniversalToken};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::DepositSecurity { amount } => execute_deposit_security(deps, env, info, amount),
        ExecuteMsg::WithdrawSecurity { amount } => {
            execute_withdraw_security(deps, env, info, amount)
        }
        ExecuteMsg::Stake { owner, amount } => execute_stake(deps, env, info, owner, amount),
        ExecuteMsg::WithdrawPrincipal { index } => {
            execute_withdraw_principal(deps, env, info, index)
        }
        ExecuteMsg::WithdrawRewards { index } => execute_withdraw_rewards(deps, env, info, index),
        ExecuteMsg::WithdrawRewardsBatch { indexes } => {
            execute_claim_rewards_batch(deps, env, info, indexes)
        }
        ExecuteMsg::Collect {
            is_native_token,
            token,
        } => execute_collect(deps, env, info, is_native_token, token),
        ExecuteMsg::GrantRole { role, account } => {
            execute_grant_role(deps, env, info, role, account)
        }
    }
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

fn execute_deposit_security(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    get_access_control().only_provider(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    if config.apy == Uint128::zero() {
        return Err(ContractError::Forbidden);
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;
    asset_info.total_security_deposit += amount;
    ASSET_INFO.save(deps.storage, &asset_info)?;

    let msgs =
        get_univeral_token(deps.storage)?.receive_token(deps.as_ref(), &env, &info, amount)?;

    Ok(Response::new()
        .add_messages(msgs)
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
    get_access_control().only_provider(deps.storage, &info.sender)?;
    let config = CONFIG.load(deps.storage)?;
    if config.apy == Uint128::zero() {
        return Err(ContractError::Forbidden);
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;
    ensure!(
        asset_info.total_security_deposit > asset_info.total_rewards,
        ContractError::InsufficientFunds
    );

    let available = asset_info.total_security_deposit - asset_info.total_rewards;
    ensure!(available >= amount, ContractError::InsufficientFunds);
    asset_info.total_security_deposit -= amount;
    ASSET_INFO.save(deps.storage, &asset_info)?;

    let msgs = get_univeral_token(deps.storage)?.send_token(&info.sender, amount)?;

    Ok(Response::new()
        .add_messages(msgs)
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

    let access_control = get_access_control();
    if config.enable_staker_whitelist {
        access_control.only_staker(deps.storage, &info.sender)?;
    }

    if config.enable_beneficiary_whitelist {
        access_control.only_beneficiary(deps.storage, &owner_addr)?;
    }

    let mut asset_info = ASSET_INFO.load(deps.storage)?;

    let (principal, rewards) = if config.apy == Uint128::zero() {
        (Uint128::zero(), amount)
    } else {
        let rewards = config.calc_security_deposit_by_collateral(amount);
        ensure!(
            rewards + asset_info.total_rewards <= asset_info.total_security_deposit,
            ContractError::InsufficientFunds
        );
        (amount, rewards)
    };

    asset_info.total_principal += principal;
    asset_info.total_rewards += rewards;
    ASSET_INFO.save(deps.storage, &asset_info)?;

    let stake_info = StakeInfo {
        owner: owner_addr.clone(),
        start_time: env.block.time.seconds().into(),
        total_principal: principal,
        withdrawn_principal: Uint128::zero(),
        total_rewards: rewards,
        withdrawn_rewards: Uint128::zero(),
    };

    STAKE_RECORDS.push(deps.storage, &stake_info)?;
    let idx = STAKE_RECORDS.len(deps.storage)? - 1;
    STAKER_INDEXES.update(deps.storage, &owner_addr, |indexes| -> StdResult<_> {
        Ok(indexes.map_or_else(
            || vec![idx],
            |mut i_vec| {
                i_vec.push(idx);
                i_vec
            },
        ))
    })?;

    let msgs =
        get_univeral_token(deps.storage)?.receive_token(deps.as_ref(), &env, &info, amount)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "stake")
        .add_attribute("owner", owner_addr)
        .add_attribute("principal", principal)
        .add_attribute("rewards", rewards)
        .add_attribute("start_time", stake_info.start_time)
        .add_attribute("lock_period", config.lock_period)
        .add_attribute("index", idx.to_string()))
}

fn execute_withdraw_principal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let mut stake_info = STAKE_RECORDS.get(deps.storage, index)?;
    if stake_info.owner != info.sender {
        return Err(ContractError::Unauthorized);
    }

    let config = CONFIG.load(deps.storage)?;
    let withdrawable_principal = config.calc_withdrawable_principal(&env, &stake_info);
    if withdrawable_principal == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "withdraw_principal")
            .add_attribute("owner", info.sender)
            .add_attribute("amount", withdrawable_principal)
            .add_attribute("index", index.to_string()));
    }

    stake_info.withdrawn_principal += withdrawable_principal;
    STAKE_RECORDS.set(deps.storage, index, &stake_info)?;
    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.withdrawn_principal += withdrawable_principal;
        Ok(info)
    })?;

    let msgs =
        get_univeral_token(deps.storage)?.send_token(&info.sender, withdrawable_principal)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_principal")
        .add_attribute("owner", info.sender)
        .add_attribute("amount", withdrawable_principal)
        .add_attribute("index", index.to_string()))
}

fn execute_withdraw_rewards(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    index: u32,
) -> Result<Response, ContractError> {
    let mut stake_info = STAKE_RECORDS.get(deps.storage, index)?;
    if stake_info.owner != info.sender {
        return Err(ContractError::Unauthorized);
    }

    let config = CONFIG.load(deps.storage)?;
    let withdrawable_rewards = config.calc_withdrawable_rewards(&env, &stake_info);
    if withdrawable_rewards == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "withdraw_rewards")
            .add_attribute("owner", info.sender)
            .add_attribute("amount", withdrawable_rewards)
            .add_attribute("index", index.to_string()));
    }

    stake_info.withdrawn_rewards += withdrawable_rewards;
    STAKE_RECORDS.set(deps.storage, index, &stake_info)?;
    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.withdrawn_rewards += withdrawable_rewards;
        Ok(info)
    })?;

    let msgs = get_univeral_token(deps.storage)?.send_token(&info.sender, withdrawable_rewards)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_rewards")
        .add_attribute("owner", info.sender)
        .add_attribute("amount", withdrawable_rewards)
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
    let mut total_withdrawable_rewards = Uint128::zero();
    for index in &indexes {
        let mut stake_info = STAKE_RECORDS.get(deps.storage, *index)?;
        if stake_info.owner != info.sender {
            return Err(ContractError::Unauthorized);
        }

        let withdrawble_reward = config.calc_withdrawable_rewards(&env, &stake_info);
        amounts.push(withdrawble_reward);

        if withdrawble_reward == Uint128::zero() {
            continue;
        }

        stake_info.withdrawn_rewards += withdrawble_reward;
        STAKE_RECORDS.set(deps.storage, *index, &stake_info)?;
        total_withdrawable_rewards += withdrawble_reward;
    }

    if total_withdrawable_rewards == Uint128::zero() {
        return Ok(Response::default()
            .add_attribute("action", "withdraw_rewards_batch")
            .add_attribute("owner", info.sender)
            .add_attribute("total_amount", total_withdrawable_rewards)
            .add_attribute("amounts", to_json_string(&amounts)?)
            .add_attribute("indexes", to_json_string(&indexes)?));
    }

    ASSET_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.withdrawn_rewards += total_withdrawable_rewards;
        Ok(info)
    })?;

    let msgs =
        get_univeral_token(deps.storage)?.send_token(&info.sender, total_withdrawable_rewards)?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "withdraw_rewards_batch")
        .add_attribute("owner", info.sender)
        .add_attribute("total_amount", total_withdrawable_rewards)
        .add_attribute("amounts", to_json_string(&amounts)?)
        .add_attribute("indexes", to_json_string(&indexes)?))
}

fn execute_collect(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    is_native_token: bool,
    token: String,
) -> Result<Response, ContractError> {
    if !is_native_token {
        deps.api.addr_validate(token.as_str())?;
    }

    get_access_control().only_admin(deps.storage, &info.sender)?;
    let token_config = TOKEN_CONFIG.load(deps.storage)?;
    let new_token = TokenConfig {
        is_native: is_native_token,
        token: token.clone(),
    };

    let held_token = if token_config == new_token {
        let config = CONFIG.load(deps.storage)?;
        let asset_info = ASSET_INFO.load(deps.storage)?;
        if config.apy == Uint128::zero() {
            asset_info.total_rewards - asset_info.withdrawn_rewards
        } else {
            asset_info.total_principal + asset_info.total_security_deposit
                - asset_info.withdrawn_rewards
                - asset_info.withdrawn_principal
        }
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
