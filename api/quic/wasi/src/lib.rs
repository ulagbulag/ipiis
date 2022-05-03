pub extern crate ipiis_api_quic_common as common;

use core::pin::Pin;

use ipiis_api_quic_common::{opcode::Opcode, Ipiis};
use ipis::{
    async_trait::async_trait,
    core::{account::AccountRef, anyhow::Result},
    tokio::io::AsyncRead,
};

mod intrinsics;

pub struct IpiisClient {
    cid: u64,
}

impl IpiisClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cid: unsafe { crate::intrinsics::ipiis_client_new() },
        })
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Opcode = Opcode;

    fn account_me(&self) -> AccountRef {
        todo!()
    }

    fn account_primary(&self) -> Option<AccountRef> {
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
