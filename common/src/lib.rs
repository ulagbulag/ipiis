use core::pin::Pin;

use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, Verifier},
        anyhow::Result,
        metadata::Metadata,
        signature::SignatureSerializer,
    },
    pin::{Pinned, PinnedInner},
    rkyv::{
        de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
        Deserialize, Serialize,
    },
    tokio::io::{AsyncRead, AsyncReadExt},
};

#[async_trait]
pub trait Ipiis {
    type Opcode: Send + Sync;

    fn account_me(&self) -> &Account;

    fn account_primary(&self) -> Result<AccountRef>;

    fn sign<T>(&self, target: AccountRef, msg: T) -> Result<GuaranteeSigned<T>>
    where
        T: Archive + Serialize<SignatureSerializer> + Send,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Metadata::builder().build(self.account_me(), target, msg)
    }

    async fn call<'res, Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: GuaranteeSigned<Req>,
    ) -> Result<Pinned<GuaranteeSigned<Res>>>
    where
        Req: Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq
            + Send
            + Sync,
        <Req as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        Res: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq + Send,
        <Res as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
            + Deserialize<Res, SharedDeserializeMap>
            + ::core::fmt::Debug
            + PartialEq,
    {
        // verify data
        let () = msg.verify(Some(*target))?;

        // send data
        let msg = ::ipis::rkyv::to_bytes::<_, SERIALIZER_HEAP_SIZE>(&msg)?;
        let hint = Some(::core::mem::size_of::<Res>());

        // recv data
        let bytes = self
            .call_raw_to_end(opcode, target, &mut msg.as_ref(), hint)
            .await?;

        // unpack data
        let res = PinnedInner::<GuaranteeSigned<Res>>::new(bytes)?;

        // verify data
        let () = res.verify(Some(self.account_me().account_ref()))?;

        Ok(res)
    }

    async fn call_permanent<'res, Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: Req,
    ) -> Result<Pinned<GuaranteeSigned<Res>>>
    where
        Req: Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq
            + Send
            + Sync,
        <Req as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        Res: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq + Send,
        <Res as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
            + Deserialize<Res, SharedDeserializeMap>
            + ::core::fmt::Debug
            + PartialEq,
    {
        let msg = self.sign(*target, msg)?;

        self.call(opcode, target, msg).await
    }

    async fn call_deserialized<Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: GuaranteeSigned<Req>,
    ) -> Result<GuaranteeSigned<Res>>
    where
        Req: Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq
            + Send
            + Sync,
        <Req as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        Res: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq + Send,
        <Res as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
            + Deserialize<Res, SharedDeserializeMap>
            + ::core::fmt::Debug
            + PartialEq,
        GuaranteeSigned<Res>: Archive,
        <GuaranteeSigned<Res> as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
    {
        self.call(opcode, target, msg)
            .await
            .and_then(|e: Pinned<GuaranteeSigned<Res>>| e.deserialize_into())
    }

    async fn call_permanent_deserialized<Req, Res>(
        &self,
        opcode: <Self as Ipiis>::Opcode,
        target: &AccountRef,
        msg: Req,
    ) -> Result<GuaranteeSigned<Res>>
    where
        Req: Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq
            + Send
            + Sync,
        <Req as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        Res: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq + Send,
        <Res as Archive>::Archived: for<'a> CheckBytes<DefaultValidator<'a>>
            + Deserialize<Res, SharedDeserializeMap>
            + ::core::fmt::Debug
            + PartialEq,
    {
        let msg = self.sign(*target, msg)?;

        self.call_deserialized::<Req, Res>(opcode, target, msg)
            .await
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

#[macro_export]
macro_rules! external_call {
    (
        account: $account:expr,
        call: $call:expr,
        response: $ty:ty => $kind:ident,
    ) => {
        external_call!(
            account: $account,
            call: $call,
            response: $ty => $kind,
            items: {},
        )
    };
    (
        account: $account:expr,
        call: $call:expr,
        response: $ty:ty => $kind:ident,
        items: { $( $items:ident ),* },
    ) => {{
        let __res: ::ipis::core::account::GuaranteeSigned<$ty> = $call;

        // verify response
        let () = ::ipis::core::account::Verifier::verify(&__res, Some($account))?;

        // unpack response
        match __res.data.data {
            <$ty>::$kind { $( $items, )* .. } => ( $( $items, )* ),
            _ => ::ipis::core::anyhow::bail!("failed to parse response"),
        }
    }};
}
