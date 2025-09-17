use crate::error::ContractError;
use crate::state::{add_held_funds, PENDING_SUBMSG_PAYLOAD, SHARE_INFOS};
use cosmwasm_std::{entry_point, from_json, DepsMut, Env, Event, Reply, Response, SubMsgResult, Uint128};

pub const EXECUTE_CLAIM_STAKE_REWARDS__WITHDRAW_REWARDS: u64 = 1;
pub const EXECUTE_CLAIM_STAKE_REWARDS_BATCH__WITHDRAW_REWARDS_BATCH: u64 = 2;
pub const EXECUTE_CLAIM_STAKE_PRINCIPAL__WITHDRAW_PRINCIPAL: u64 = 3;

#[cfg_attr(not(feature = "library"), entry_point)]
fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.result {
        SubMsgResult::Err(err) => return Err(ContractError::SubMsgFailed { msg: err }),
        _ => {}
    };

    match msg.id {
        EXECUTE_CLAIM_STAKE_REWARDS__WITHDRAW_REWARDS => reply_claim_stake_rewards(deps, env, msg),
        EXECUTE_CLAIM_STAKE_REWARDS_BATCH__WITHDRAW_REWARDS_BATCH => {
            reply_claim_stake_rewards_batch(deps, env, msg)
        }
        EXECUTE_CLAIM_STAKE_PRINCIPAL__WITHDRAW_PRINCIPAL => {
            reply_claim_stake_principal(deps, env, msg)
        }
        id => Err(ContractError::UnknownReplyId { id }),
    }
}

fn reply_claim_stake_rewards(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amount = get_value_from_event(&resp.events, "withdraw_rewards", "amount").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in withdraw_rewards"),
        },
    )?;

    let payload = PENDING_SUBMSG_PAYLOAD.load(deps.storage)?;
    let amount: Uint128 = amount.parse()?;
    let share_id = from_json(payload)?;
    SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
        let mut share_info = old.ok_or(ContractError::InvalidShareId {})?;
        share_info.claimed_rewards += amount;
        Ok(share_info)
    })?;

    add_held_funds(deps.storage, amount)?;
    Ok(Response::default())
}

fn reply_claim_stake_rewards_batch(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amounts = get_value_from_event(&resp.events, "withdraw_rewards_batch", "amounts").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in withdraw_rewards_batch"),
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

    let mut total_amount = Uint128::zero();
    for (share_id, amount) in share_ids.into_iter().zip(amounts.into_iter()) {
        SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
            let mut share_info = old.ok_or(ContractError::InvalidShareId {})?;
            share_info.claimed_rewards += amount;
            Ok(share_info)
        })?;

        total_amount += amount;
    }

    add_held_funds(deps.storage, total_amount)?;
    Ok(Response::default())
}

fn reply_claim_stake_principal(
    deps: DepsMut,
    _env: Env,
    msg: Reply,
) -> Result<Response, ContractError> {
    let resp = msg.result.unwrap();
    let amount = get_value_from_event(&resp.events, "withdraw_principal", "amount").ok_or(
        ContractError::InnerError {
            msg: format!("cannot get amount from event in withdraw_principal"),
        },
    )?;

    let amount: Uint128 = amount.parse()?;
    let payload = PENDING_SUBMSG_PAYLOAD.load(deps.storage)?;
    let share_id = from_json(payload)?;
    SHARE_INFOS.update(deps.storage, share_id, |old| -> Result<_, ContractError> {
        let mut share_info = old.ok_or(ContractError::InvalidShareId {})?;
        share_info.claimed_principal += amount;
        Ok(share_info)
    })?;

    add_held_funds(deps.storage, amount)?;
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
