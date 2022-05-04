use core::pin::Pin;

use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{account::AccountRef, anyhow::Result},
    tokio::io::AsyncRead,
};

use crate::common::opcode::Opcode;

pub struct IpiisClient {
    cid: u64,
}

impl IpiisClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cid: unsafe { super::intrinsics::ipiis_client_new() },
        })
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Opcode = Opcode;

    fn account_me(&self) -> AccountRef {
        todo!()
    }

    fn account_primary(&self) -> Result<AccountRef> {
        todo!()
    }

    async fn call_raw<Req>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &mut Req,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        todo!()
    }
}
