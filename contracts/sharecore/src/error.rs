use cosmwasm_std::StdError;
use thiserror::Error;
use access_control::AccessControlError;
use universal_token::UniversalTokenError;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    UniversalTokenError(#[from] UniversalTokenError),
    
    #[error("{key}")]
    InvalidParamter { key: String },

    #[error("Unauthorized")]
    Unauthorized,

    #[error("AmountExceedsWithdrawable")]
    AmountExceedsWithdrawable,

    #[error("AmountExceedsBalance")]
    AmountExceedsBalance,

    #[error("Insufficient reward")]
    InsufficientReward,

    #[error("Insufficient principal")]
    InsufficientPrincipal,

    #[error("Holder already registered")]
    HolderAlreadyRegistered,

    #[error("Stakecore already initialized")]
    StakecoreAlreadyInitialized,

    #[error("InvalidShareId")]
    InvalidShareId,

    #[error("Inner error: {msg}")]
    InnerError { msg: String },

    #[error("Unknown ReplyId: {id}")]
    UnknownReplyId { id: u64 },

    #[error("SubMsg failed: {msg}")]
    SubMsgFailed { msg: String },

    #[error("No excessive tokens")]
    NoExcessTokens,

    #[error("{0}")]
    AccessControl(#[from] AccessControlError),
}
