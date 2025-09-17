use cosmwasm_std::storage_keys::namespace_with_key;
use cosmwasm_std::{Order, StdError, StdResult, Storage};
use cw_storage_plus::{Map, Namespace};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

const LEN_KEY: &[u8] = b"len";

/// A growable array backed by `Map<u32, T>` under the given `prefix`.
///
/// - Elements are stored at keys `prefix/<index as big-endian bytes>`.
/// - Length is stored at key `prefix/len`.
pub struct Vector<T> {
    namespace: Namespace,
    item_type: PhantomData<T>,
}

impl<T> Vector<T>
where
    T: Serialize + DeserializeOwned,
{
    pub const fn new(prefix: &'static str) -> Self {
        Self {
            namespace: Namespace::from_static_str(prefix),
            item_type: PhantomData,
        }
    }

    pub fn push(&self, store: &mut dyn Storage, item: &T) -> StdResult<()> {
        let len = self.len(store)?;
        let new_len = len
            .checked_add(1)
            .ok_or_else(|| StdError::generic_err("Vector overflow"))?;
        self.as_map().save(store, len, item)?;
        self.set_len(store, new_len);
        Ok(())
    }

    pub fn pop(&self, store: &mut dyn Storage) -> StdResult<T> {
        let len = self.len(store)?;
        if len == 0 {
            return Err(StdError::generic_err("Pop on empty Vector"));
        }

        let item = self.as_map().load(store, len - 1)?;
        self.as_map().remove(store, len - 1);
        self.set_len(store, len - 1);
        Ok(item)
    }

    pub fn get(&self, store: &dyn Storage, key: u32) -> StdResult<T> {
        let len = self.len(store)?;
        if key >= len {
            return Err(StdError::generic_err(format!("Key {} out of bounds!", key)));
        }

        self.as_map().load(store, key)
    }

    pub fn set(&self, store: &mut dyn Storage, key: u32, item: &T) -> StdResult<()> {
        let len = self.len(store)?;
        if key > len {
            return Err(StdError::generic_err(format!("Key {} out of bounds!", key)));
        }

        self.as_map().save(store, key, item)?;
        if key == len {
            self.set_len(store, len + 1);
        }

        Ok(())
    }

    pub fn as_map(&self) -> Map<u32, T> {
        Map::new_dyn(self.namespace.clone())
    }

    pub fn len(&self, store: &dyn Storage) -> StdResult<u32> {
        let full_key = namespace_with_key(&[self.namespace.as_slice()], LEN_KEY);
        match store.get(&full_key) {
            Some(buf) => Ok(u32::from_be_bytes(
                buf.as_slice()
                    .try_into()
                    .map_err(|e| StdError::parse_err("u32", e))?,
            )),
            None => Ok(0),
        }
    }

    #[inline]
    fn set_len(&self, store: &mut dyn Storage, len: u32) {
        let full_key = namespace_with_key(&[self.namespace.as_slice()], LEN_KEY);
        store.set(&full_key, &len.to_be_bytes());
    }

    pub fn range<'c>(
        &self,
        store: &'c dyn Storage,
    ) -> Box<dyn Iterator<Item = StdResult<(u32, T)>> + 'c>
    where
        T: 'c,
    {
        self.as_map().range(store, None, None, Order::Ascending)
    }
}
