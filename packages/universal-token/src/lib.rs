use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    ensure, to_json_binary, Addr, BalanceResponse, BankMsg, BankQuery, Coin, CosmosMsg, Deps, Env,
    MessageInfo, QueryRequest, StdError, Uint128, WasmMsg, WasmQuery,
};
use cw20::{
    AllowanceResponse, BalanceResponse as Cw20BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg,
};
use thiserror::Error;

#[cw_serde]
pub struct TokenConfig {
    pub is_native: bool,
    pub token: String,
}

pub struct UniversalToken {
    pub config: TokenConfig,
}

impl UniversalToken {
    pub fn new(config: TokenConfig) -> Self {
        UniversalToken { config }
    }

    pub fn is_native(&self) -> bool {
        self.config.is_native
    }

    pub fn send_token(
        &self,
        account: &Addr,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, UniversalTokenError> {
        if self.is_native() {
            // 发送原生币
            let msg = CosmosMsg::Bank(BankMsg::Send {
                to_address: account.to_string(),
                amount: vec![Coin {
                    denom: self.config.token.clone(),
                    amount,
                }],
            });
            Ok(vec![msg])
        } else {
            // 发送 CW20
            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.config.token.clone(),
                msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: account.to_string(),
                    amount,
                })?,
                funds: vec![],
            });
            Ok(vec![msg])
        }
    }

    pub fn receive_token(
        &self,
        deps: Deps,
        env: &Env,
        info: &MessageInfo,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, UniversalTokenError> {
        if self.is_native() {
            ensure!(
                info.funds.len() == 1,
                UniversalTokenError::IllegalMsgFunds {}
            );
            let Coin {
                denom,
                amount: got_amt,
            } = info.funds[0].clone();
            let token = self.config.token.clone();
            ensure!(
                denom == token,
                UniversalTokenError::WrongDenom {
                    expect: token,
                    got: denom
                }
            );

            ensure!(
                got_amt == amount,
                UniversalTokenError::WrongAmount {
                    expect: amount,
                    got: got_amt
                }
            );

            Ok(vec![])
        } else {
            ensure!(
                info.funds.len() == 0,
                UniversalTokenError::IllegalMsgFunds {}
            );

            let allowance: AllowanceResponse = deps.querier.query_wasm_smart(
                self.config.token.clone(),
                &Cw20QueryMsg::Allowance {
                    owner: info.sender.to_string(),
                    spender: env.contract.address.to_string(),
                },
            )?;

            ensure!(
                !allowance.expires.is_expired(&env.block),
                UniversalTokenError::WrongAmount {
                    expect: amount,
                    got: Uint128::zero()
                }
            );

            ensure!(
                allowance.allowance >= amount,
                UniversalTokenError::WrongAmount {
                    expect: amount,
                    got: allowance.allowance,
                }
            );

            let msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: self.config.token.clone(),
                msg: to_json_binary(&Cw20ExecuteMsg::TransferFrom {
                    owner: info.sender.to_string(),
                    recipient: env.contract.address.to_string(),
                    amount,
                })?,
                funds: vec![],
            });

            Ok(vec![msg])
        }
    }

    pub fn balance(&self, deps: Deps, env: &Env) -> Result<Uint128, UniversalTokenError> {
        if self.is_native() {
            let bal: BalanceResponse =
                deps.querier.query(&QueryRequest::Bank(BankQuery::Balance {
                    address: env.contract.address.to_string(),
                    denom: self.config.token.clone(),
                }))?;
            Ok(bal.amount.amount)
        } else {
            let bal: Cw20BalanceResponse =
                deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                    contract_addr: self.config.token.clone(),
                    msg: to_json_binary(&Cw20QueryMsg::Balance {
                        address: env.contract.address.to_string(),
                    })?,
                }))?;
            Ok(bal.balance)
        }
    }
}

#[derive(Error, Debug)]
pub enum UniversalTokenError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Illegal msg funds")]
    IllegalMsgFunds,

    #[error("wrong denom: expect {expect}, got {got}")]
    WrongDenom { expect: String, got: String },

    #[error("wrong amount: expect {expect}, got {got}")]
    WrongAmount { expect: Uint128, got: Uint128 },
}
