use core::{marker::PhantomData, str::FromStr};
use std::{net::ToSocketAddrs, sync::Arc};

use ipis::core::{
    account::{Account, AccountRef},
    anyhow::{anyhow, bail, Result},
    value::hash::Hash,
};

#[derive(Clone, Debug)]
pub struct RarpClient<Address> {
    pub account_me: Arc<Account>,
    pub account_ref: Arc<AccountRef>,
    table: sled::Db,
    _address: PhantomData<Address>,
}

impl<Address> RarpClient<Address> {
    pub fn new<P>(account_me: Account, db_path: P) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        Ok(Self {
            account_ref: account_me.account_ref().into(),
            account_me: account_me.into(),
            // TODO: allow to store in specific directory
            table: sled::open(::tempfile::tempdir()?.path().join(db_path))?,
            _address: Default::default(),
        })
    }

    pub fn get(&self, kind: Option<&Hash>, target: &AccountRef) -> Result<Option<Address>>
    where
        Address: FromStr + ToSocketAddrs,
        <Address as FromStr>::Err: ::std::error::Error + Send + Sync + 'static,
    {
        let key = self.to_key_canonical(kind, Some(target));

        match self.table.get(key)? {
            Some(address) => Ok(Some(String::from_utf8(address.to_vec())?.parse()?)),
            None => Ok(None),
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
        Address: ::std::fmt::Debug + ToSocketAddrs + ToString,
    {
        // verify address
        match address
            .to_socket_addrs()
            .map_err(|e| anyhow!("failed to parse the socket address: {address:?}: {e}"))?
            .next()
        {
            Some(address) => {
                let key = self.to_key_canonical(kind, Some(target));

                self.table
                    .insert(key, address.to_string().into_bytes())
                    .map(|_| ())
                    .map_err(Into::into)
            }
            None => bail!("failed to parse the socket address: {address:?}"),
        }
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

        let kind: Vec<u8> = kind.cloned().map(Into::into).unwrap_or_default();
        let account = account
            .map(|e| e.as_bytes().as_ref())
            .unwrap_or_else(|| &[]);

        [&[flag], kind.as_slice(), account].concat()
    }
}
