use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct Ownership {
    pub owner: Addr,
    pub pending_owner: Option<Addr>,
}

impl Ownership {
    pub fn new(owner: Addr) -> Self {
        Ownership {
            owner: owner,
            pending_owner: None,
        }
    }

    pub fn validate(&self, owner: &Addr) -> bool {
        if self.owner == *owner {
            true
        } else {
            false
        }
    }

    pub fn accept_ownership(&mut self, pending_owner: &Addr) -> bool {
        if self.pending_owner.as_ref() == Some(pending_owner) {
            self.owner = self.pending_owner.take().unwrap();
            true
        } else {
            false
        }
    }
}
