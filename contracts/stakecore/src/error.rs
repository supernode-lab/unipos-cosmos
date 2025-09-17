use access_control::AccessControlError;
use cosmwasm_std::StdError;
use thiserror::Error;
use universal_token::UniversalTokenError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("forbid")]
    Forbidden,

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

    #[error("{0}")]
    UniversalTokenError(#[from] UniversalTokenError),

    #[error("No excessive tokens")]
    NoExcessTokens,

    #[error("{0}")]
    AccessControl(#[from] AccessControlError),
}
