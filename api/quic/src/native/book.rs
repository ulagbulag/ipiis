use core::{marker::PhantomData, str::FromStr};
use std::sync::Arc;

use ipis::core::{
    account::{Account, AccountRef},
    anyhow::{bail, Result},
    value::hash::Hash,
};

#[derive(Clone, Debug)]
pub struct AddressBook<Address> {
    pub(crate) account_me: Arc<Account>,
    table: sled::Db,
    _address: PhantomData<Address>,
}

impl<Address> AddressBook<Address> {
    pub fn new<P>(account_me: Account, book_path: P) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        Ok(Self {
            account_me: account_me.into(),
            // TODO: allow to store in specific directory
            table: sled::open(::tempfile::tempdir()?.path().join(book_path))?,
            _address: Default::default(),
        })
    }

    pub fn get(&self, target: &AccountRef) -> Result<Option<Address>>
    where
        Address: FromStr,
        <Address as FromStr>::Err: ::std::error::Error + Send + Sync + 'static,
    {
        match self.table.get(target.as_bytes())? {
            Some(address) => Ok(Some(String::from_utf8(address.to_vec())?.parse()?)),
            None => {
                if &self.account_me.account_ref() == target {
                    bail!("cannot get the address myself");
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn get_primary(&self, kind: Option<&Hash>) -> Result<Option<AccountRef>> {
        let kind = self.unwrap_kind(kind);

        match self.table.get(&**kind)? {
            Some(address) => Ok(Some(String::from_utf8(address.to_vec())?.parse()?)),
            None => Ok(None),
        }
    }

    pub fn set(&self, target: &AccountRef, address: &Address) -> Result<()>
    where
        Address: ToString,
    {
        self.table
            .insert(target.as_bytes(), address.to_string().into_bytes())
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn set_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        let kind = self.unwrap_kind(kind);

        self.table
            .insert(&**kind, account.to_string().into_bytes())
            .map(|_| ())
            .map_err(Into::into)
    }

    fn unwrap_kind(&self, kind: Option<&Hash>) -> Hash {
        kind.copied().unwrap_or_else(|| Hash::with_bytes(&[]))
    }
}
