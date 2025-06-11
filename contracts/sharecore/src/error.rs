use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient reward")]
    InsufficientReward {},

    #[error("Insufficient principal")]
    InsufficientPrincipal {},

    #[error("No reward")]
    NoReward {},

    #[error("No principal")]
    NoPrincipal {},

    #[error("Holder already registered")]
    HolderAlreadyRegistered {},

    #[error("Stakecore already initialized")]
    StakecoreAlreadyInitialized {},

    #[error("ShareInfo not found")]
    ShareInfoNotFound {},

    #[error("Inner error: {msg}")]
    InnerError { msg: String },

    #[error("Unknown ReplyId: {id}")]
    UnknownReplyId { id: u64 },

    #[error("SubMsg failed: {msg}")]
    SubMsgFailed { msg: String },
}
