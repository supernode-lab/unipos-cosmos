use cosmwasm_std::{Addr, StdError, Storage};
use cw_storage_plus::Map;
use thiserror::Error;

pub const DEFAULT_ADMIN_ROLE: &'static str = "__DEFAULT_ADMIN__";

pub struct AccessControl {
    roles: Map<(String, Addr), ()>,
    admin_roles: Map<String, String>,
}

impl AccessControl {
    pub fn new(roles: Map<(String, Addr), ()>, admin_roles: Map<String, String>) -> AccessControl {
        AccessControl { roles, admin_roles }
    }

    pub fn grant_role(
        &self,
        storage: &mut dyn Storage,
        admin: &Addr,
        role: &str,
        account: &Addr,
    ) -> Result<(), AccessControlError> {
        self.check_is_admin(storage, role, admin)?;
        self._grant_role(storage, role, account)?;
        Ok(())
    }

    pub fn revoke_role(
        &self,
        storage: &mut dyn Storage,
        admin: &Addr,
        role: &str,
        account: &Addr,
    ) -> Result<(), AccessControlError> {
        self.check_is_admin(storage, role, admin)?;
        self._revoke_role(storage, role, account);
        Ok(())
    }

    pub fn set_role_admin(
        &self,
        storage: &mut dyn Storage,
        role: &str,
        admin: &String,
    ) -> Result<(), StdError> {
        self.admin_roles.save(storage, role.to_string(), admin)?;
        Ok(())
    }

    pub fn check_is_admin(
        &self,
        storage: &dyn Storage,
        role: &str,
        admin: &Addr,
    ) -> Result<(), AccessControlError> {
        self.check_role(storage, &self.get_role_admin(storage, role)?, admin)
    }

    pub fn check_role(
        &self,
        storage: &dyn Storage,
        role: &str,
        account: &Addr,
    ) -> Result<(), AccessControlError> {
        if self.has_role(storage, &role, account) {
            Ok(())
        } else {
            Err(AccessControlError::UnauthorizedAccount {
                account: account.clone(),
                needed_role: role.to_string(),
            })
        }
    }

    pub fn get_role_admin(&self, storage: &dyn Storage, role: &str) -> Result<String, StdError> {
        let admin_role = self.admin_roles.may_load(storage, role.to_string())?;
        if let Some(admin_role) = admin_role {
            Ok(admin_role)
        } else {
            Ok(DEFAULT_ADMIN_ROLE.to_string())
        }
    }

    pub fn has_role(&self, storage: &dyn Storage, role: &str, account: &Addr) -> bool {
        self.roles.has(storage, (role.to_string(), account.clone()))
    }

    pub fn _grant_role(
        &self,
        storage: &mut dyn Storage,
        role: &str,
        account: &Addr,
    ) -> Result<bool, StdError> {
        if self.has_role(storage, role, account) {
            Ok(false)
        } else {
            self.roles
                .save(storage, (role.to_string(), account.clone()), &())?;
            Ok(true)
        }
    }

    pub fn _revoke_role(&self, storage: &mut dyn Storage, role: &str, account: &Addr) -> bool {
        if !self.has_role(storage, role, account) {
            false
        } else {
            self.roles
                .remove(storage, (role.to_string(), account.clone()));
            true
        }
    }
}

#[derive(Error, Debug)]
pub enum AccessControlError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized account {account}, requires role: {needed_role}")]
    UnauthorizedAccount { account: Addr, needed_role: String },
}
