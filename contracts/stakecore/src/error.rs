use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Stake amount too low")]
    InsufficientStakeAmount,

    #[error("Lock period has not ended yet")]
    LockPeriodNotEnded,

    #[error("Already unstaked")]
    AlreadyUnstaked,

    #[error("Invalid input")]
    InvalidInput,

    #[error("No locked funds")]
    NoLockedFunds,
}
