use std::io::Cursor;

use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef},
        anyhow::Result,
        value::hash::Hash,
    },
    env::{infer, Infer},
};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IpiisClient {
    cid: u64,
}

#[async_trait]
impl<'a> Infer<'a> for IpiisClient {
    type GenesisArgs = Option<AccountRef>;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_account_primary").ok();

        Self::new(account_me, account_primary)
    }

    async fn genesis(
        account_primary: <Self as Infer>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        let account_primary = account_primary.or_else(|| infer("ipiis_account_primary").ok());

        // generate an account
        let account = Account::generate();

        // init an endpoint
        Self::new(account, account_primary)
    }
}

impl IpiisClient {
    pub fn new(account_me: Account, account_primary: Option<AccountRef>) -> Result<Self> {
        Ok(Self {
            cid: unsafe { super::intrinsics::ipiis_client_new() },
        })
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Address = ::std::net::SocketAddr;
    type Reader = Cursor<Vec<u8>>;
    type Writer = Vec<u8>;

    fn account_me(&self) -> &Account {
        unsafe { &*(super::intrinsics::ipiis_client_account_me() as *const Account) }
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        todo!()
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        todo!()
    }

    async fn get_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<<Self as Ipiis>::Address> {
        todo!()
    }

    async fn set_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        todo!()
    }

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)> {
        todo!()
    }
}
