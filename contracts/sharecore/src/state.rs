pub use crate::types::*;
use cosmwasm_std::{Addr, StdError, Storage, Uint128};
use cw_storage_plus::{Item, Map};
pub use universal_token::TokenConfig;

pub const CONFIG: Item<Config> = Item::new("config");

pub const HELD_FUNDS: Item<Uint128> = Item::new("held_funds");
pub const SHARE_INFOS: Map<u32, ShareInfo> = Map::new("share_infos");
pub const SHARE_INFO_INDEXES: Item<Vec<u32>> = Item::new("share_info_indexes");

pub const SHAREHOLDER_INFOS: Map<(&Addr, u32), ShareholderInfo> = Map::new("shareholder_infos");
pub const PENDING_SUBMSG_PAYLOAD: Item<Vec<u8>> = Item::new("pending_submsg_payload");

pub fn add_held_funds(storage: &mut dyn Storage, amount: Uint128) -> Result<(), StdError> {
    HELD_FUNDS.update(storage, |data| Result::<_, StdError>::Ok(data + amount))?;
    Ok(())
}

pub fn sub_held_funds(storage: &mut dyn Storage, amount: Uint128) -> Result<(), StdError> {
    HELD_FUNDS.update(storage, |data| Result::<_, StdError>::Ok(data - amount))?;
    Ok(())
}

pub mod ut {
    use cosmwasm_std::{StdError, Storage};
    use cw_storage_plus::Item;
    use universal_token::{TokenConfig, UniversalToken};

    pub const TOKEN_CONFIG: Item<TokenConfig> = Item::new("token_config");

    pub fn get_univeral_token(storage: &dyn Storage) -> Result<UniversalToken, StdError> {
        Ok(UniversalToken::new(TOKEN_CONFIG.load(storage)?))
    }
}

pub mod ac {
    use crate::error::ContractError;
    use access_control::DEFAULT_ADMIN_ROLE;
    use cosmwasm_std::{Addr, Storage};
    use cw_storage_plus::Map;
    use std::ops::Deref;

    const ROLES: Map<(String,Addr), ()> = Map::new("roles");
    const ADMIN_ROLES: Map<String, String> = Map::new("admin_roles");
    pub const SHAREHOLDER_ROLE: &'static str = "shareholder";

    pub struct AccessControl(access_control::AccessControl);

    pub fn get_access_control() -> AccessControl {
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

        pub fn only_shareholder(
            &self,
            storage: &mut dyn Storage,
            account: &Addr,
        ) -> Result<(), ContractError> {
            self.check_role(storage, SHAREHOLDER_ROLE, account)?;
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
