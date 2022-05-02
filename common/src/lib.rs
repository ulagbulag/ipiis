use core::pin::Pin;
use std::marker::PhantomData;

use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{
        account::AccountRef,
        anyhow::{anyhow, Result},
    },
    rkyv::{
        validation::{validators::DefaultValidator, CheckTypeError},
        Archive, Deserialize, Fallible, Serialize,
    },
    tokio::io::{AsyncRead, AsyncReadExt},
};

pub struct MaybeArchive<T> {
    _type: PhantomData<T>,
    raw: Vec<u8>,
}

impl<'a, T> MaybeArchive<T>
where
    T: Archive,
    <T as Archive>::Archived: CheckBytes<DefaultValidator<'a>>,
{
    pub fn check_archived_root(
        &'a self,
    ) -> Result<
        &'a <T as Archive>::Archived,
        CheckTypeError<<T as Archive>::Archived, DefaultValidator<'a>>,
    > {
        ::ipis::rkyv::check_archived_root::<T>(&self.raw)
    }
}

#[async_trait]
pub trait Ipiis {
    type Opcode: Send + Sync;

    fn account_me(&self) -> AccountRef;

    fn account_primary(&self) -> Option<AccountRef>;

    async fn call<'res, Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &Req,
    ) -> Result<MaybeArchive<Res>>
    where
        Req: Serialize<Serializer> + Send + Sync,
        Res: Archive + Send,
    {
        let msg = ::ipis::rkyv::to_bytes(msg)?;
        let hint = Some(::core::mem::size_of::<Res>());

        Ok(MaybeArchive {
            _type: Default::default(),
            raw: self
                .call_raw_to_end(opcode, target, &mut msg.as_ref(), hint)
                .await?,
        })
    }

    async fn call_deserialized<Req, Res, D>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: &Req,
        deserializer: &mut D,
    ) -> Result<Res>
    where
        Req: Serialize<Serializer> + Send + Sync,
        Res: Archive + Send,
        <Res as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>> + Deserialize<Res, D>,
        D: Fallible + Send + ?Sized,
        <D as Fallible>::Error: ::std::error::Error + Send + Sync,
    {
        let res = self.call::<Req, Res>(opcode, target, msg).await?;
        let res = res
            .check_archived_root()
            .map_err(|_| anyhow!("failed to parse the received address"))?;
        res.deserialize(deserializer).map_err(Into::into)
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
