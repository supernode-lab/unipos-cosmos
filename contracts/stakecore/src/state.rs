#![cfg(not(feature = "library"))]

pub use crate::types::*;
pub use access_control::*;
use bh_storage::Vector;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
pub use universal_token::TokenConfig;

pub(crate) const CONFIG: Item<Config> = Item::new("config");

pub(crate) const ASSET_INFO: Item<AssetInfo> = Item::new("asset_info");

pub(crate) const STAKE_RECORDS: Vector<StakeInfo> = Vector::new("stake_records");

pub(crate) const STAKER_INDEXES: Map<&Addr, Vec<u32>> = Map::new("staker_indexes");

pub mod ut {
    use cosmwasm_std::{StdError, Storage};
    use cw_storage_plus::Item;
    use universal_token::{TokenConfig, UniversalToken};

    pub(crate) const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");

    pub(crate) fn get_univeral_token(storage: &dyn Storage) -> Result<UniversalToken, StdError> {
        Ok(UniversalToken::new(TOKEN_CONFIG.load(storage)?))
    }
}

pub mod ac {
    use crate::ContractError;
    pub use access_control::DEFAULT_ADMIN_ROLE;
    use cosmwasm_std::{Addr, Storage};
    use cw_storage_plus::Map;
    use std::ops::Deref;

    pub(crate) const PROVIDER_ROLE: &'static str = "provider";
    pub(crate) const STAKER_ROLE: &'static str = "staker";
    pub(crate) const BENEFICIARY_ROLE: &'static str = "beneficiary";

    const ROLES: Map<(String, Addr), ()> = Map::new("roles");
    const ADMIN_ROLES: Map<String, String> = Map::new("admin_roles");

    pub(crate) struct AccessControl(access_control::AccessControl);

    pub(crate) fn get_access_control() -> AccessControl {
        AccessControl(access_control::AccessControl::new(ROLES, ADMIN_ROLES))
    }

    impl AccessControl {
        pub fn only_admin(
            &self,
            storage: &mut dyn Storage,
            account: &Addr,
        ) -> Result<(), ContractError> {
            self.check_role(storage, DEFAULT_ADMIN_ROLE, account)?;
            Ok(())
        }

        pub fn only_provider(
            &self,
            storage: &mut dyn Storage,
            account: &Addr,
        ) -> Result<(), ContractError> {
            self.check_role(storage, PROVIDER_ROLE, account)?;
            Ok(())
        }

        pub fn only_staker(
            &self,
            storage: &mut dyn Storage,
            account: &Addr,
        ) -> Result<(), ContractError> {
            self.check_role(storage, STAKER_ROLE, account)?;
            Ok(())
        }

        pub fn only_beneficiary(
            &self,
            storage: &mut dyn Storage,
            account: &Addr,
        ) -> Result<(), ContractError> {
            self.check_role(storage, BENEFICIARY_ROLE, account)?;
            Ok(())
        }
    }

    impl Deref for AccessControl {
        type Target = access_control::AccessControl;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
