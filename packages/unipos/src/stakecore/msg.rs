use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub enum ExecuteMsg {
    TransferProviderOwnership { new_provider: String },
    AcceptProviderOwnership,
    TransferAdminOwnership { new_admin: String },
    AcceptAdminOwnership,
    InitBeneficiary { bf: String },
    DepositSecurity { amount: Uint128 },
    WithdrawSecurity { amount: Uint128 },
    Stake { owner: String, amount: Uint128 },
    Unstake { index: u32 },
    ClaimReward { index: u32 },
    ClaimRewardsBatch { indexes: Vec<u32> },
    ClaimBeneficiaryReward,
    Collect,
}
