#[cfg(not(target_os = "wasi"))]
pub extern crate rustls;

pub mod common;
#[cfg(not(target_os = "wasi"))]
mod native;
#[cfg(target_os = "wasi")]
mod wasi;

use core::pin::Pin;

use bytecheck::CheckBytes;
use ipiis_common::{Ipiis, Serializer};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef, GuaranteeSigned},
        anyhow::Result,
        signature::SignatureSerializer,
    },
    pin::Pinned,
    tokio::io::AsyncRead,
};
use rkyv::{
    de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
    Deserialize, Serialize,
};

#[cfg(not(target_os = "wasi"))]
pub use self::native::*;
#[cfg(target_os = "wasi")]
pub use self::wasi::*;

impl AsRef<Self> for crate::client::IpiisClient {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<crate::client::IpiisClient> for crate::server::IpiisServer {
    fn as_ref(&self) -> &crate::client::IpiisClient {
        &self.client
    }
}

impl AsRef<Self> for crate::server::IpiisServer {
    fn as_ref(&self) -> &Self {
        self
    }
}

#[async_trait]
impl Ipiis for crate::server::IpiisServer {
    type Opcode = crate::common::opcode::Opcode;

    fn account_me(&self) -> &Account {
        self.client.account_me()
    }

    fn account_primary(&self) -> Result<AccountRef> {
        self.client.account_primary()
    }

    fn sign<T>(&self, target: AccountRef, msg: T) -> Result<GuaranteeSigned<T>>
    where
        T: Archive + Serialize<SignatureSerializer> + Send,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        self.client.sign(target, msg)
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
        self.client.call(opcode, target, msg).await
    }

    async fn call_unchecked<'res, Req, Res>(
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
        self.client.call_unchecked(opcode, target, msg).await
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
        self.client.call_permanent(opcode, target, msg).await
    }

    async fn call_permanent_unchecked<'res, Req, Res>(
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
        self.client
            .call_permanent_unchecked(opcode, target, msg)
            .await
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
        self.client.call_deserialized(opcode, target, msg).await
    }

    async fn call_deserialized_unchecked<Req, Res>(
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
        self.client
            .call_deserialized_unchecked(opcode, target, msg)
            .await
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
        self.client
            .call_permanent_deserialized(opcode, target, msg)
            .await
    }

    async fn call_permanent_deserialized_unchecked<Req, Res>(
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
        self.client
            .call_permanent_deserialized_unchecked(opcode, target, msg)
            .await
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
        self.client.call_raw(opcode, target, msg).await
    }

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
        self.client.call_raw_exact(opcode, target, msg, buf).await
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
        self.client.call_raw_to_end(opcode, target, msg, hint).await
    }
}
