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

    pub fn get(&self, kind: Option<&Hash>, target: &AccountRef) -> Result<Option<Address>>
    where
        Address: FromStr,
        <Address as FromStr>::Err: ::std::error::Error + Send + Sync + 'static,
    {
        let key = self.to_key_canonical(kind, Some(target));

        match self.table.get(key)? {
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
        let key = self.to_key_canonical(kind, None);

        match self.table.get(key)? {
            Some(address) => Ok(Some(String::from_utf8(address.to_vec())?.parse()?)),
            None => Ok(None),
        }
    }

    pub fn set(&self, kind: Option<&Hash>, target: &AccountRef, address: &Address) -> Result<()>
    where
        Address: ToString,
    {
        let key = self.to_key_canonical(kind, Some(target));

        self.table
            .insert(key, address.to_string().into_bytes())
            .map(|_| ())
            .map_err(Into::into)
    }

    pub fn set_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        let key = self.to_key_canonical(kind, None);

        self.table
            .insert(key, account.to_string().into_bytes())
            .map(|_| ())
            .map_err(Into::into)
    }

    fn to_key_canonical(&self, kind: Option<&Hash>, account: Option<&AccountRef>) -> Vec<u8> {
        #[allow(clippy::identity_op)]
        let flag = ((kind.is_some() as u8) << 1) + ((account.is_some() as u8) << 0);

        let kind = kind.map(|e| &***e).unwrap_or_else(|| &[]);
        let account = account
            .map(|e| e.as_bytes().as_ref())
            .unwrap_or_else(|| &[]);

        [&[flag], kind, account].concat()
    }
}
