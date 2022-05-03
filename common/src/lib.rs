use core::pin::Pin;

use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{account::AccountRef, anyhow::Result},
    pin::{Pinned, PinnedInner},
    rkyv::{validation::validators::DefaultValidator, Archive, Deserialize, Infallible, Serialize},
    tokio::io::{AsyncRead, AsyncReadExt},
};

#[async_trait]
pub trait Ipiis {
    type Opcode: Send + Sync;

    fn account_me(&self) -> AccountRef;

    fn account_primary(&self) -> Result<AccountRef>;

    async fn call<'res, Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &Req,
    ) -> Result<Pinned<Res>>
    where
        Req: Serialize<Serializer> + Send + Sync,
        Res: Archive + Send,
        <Res as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<Res, Infallible>,
    {
        let msg = ::ipis::rkyv::to_bytes(msg)?;
        let hint = Some(::core::mem::size_of::<Res>());

        let bytes = self
            .call_raw_to_end(opcode, target, &mut msg.as_ref(), hint)
            .await?;
        PinnedInner::<Res>::new(bytes)
    }

    async fn call_deserialized<Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &Req,
    ) -> Result<Res>
    where
        Req: Serialize<Serializer> + Send + Sync,
        Res: Archive + Send,
        <Res as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<Res, Infallible>,
    {
        self.call(opcode, target, msg)
            .await
            .and_then(|e: Pinned<Res>| e.deserialize_into())
    }

    async fn call_raw<Req>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &mut Req,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>>
    where
        Req: AsyncRead + Send + Sync + Unpin;

    async fn call_raw_exact<Req>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &mut Req,
        buf: &mut [u8],
    ) -> Result<usize>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        self.call_raw(opcode, target, msg)
            .await?
            .read_exact(buf)
            .await
            .map_err(Into::into)
    }

    async fn call_raw_to_end<Req>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &mut Req,
        hint: Option<usize>,
    ) -> Result<Vec<u8>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        let mut buf = match hint {
            Some(hint) => Vec::with_capacity(hint),
            None => Vec::default(),
        };
        self.call_raw(opcode, target, msg)
            .await?
            .read_to_end(&mut buf)
            .await?;
        Ok(buf)
    }
}

pub type Serializer = ::ipis::rkyv::ser::serializers::AllocSerializer<SERIALIZER_HEAP_SIZE>;

pub const SERIALIZER_HEAP_SIZE: usize = 4096;
