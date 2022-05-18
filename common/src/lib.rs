use core::pin::Pin;

use bytecheck::CheckBytes;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, GuarantorSigned, Signer, Verifier},
        anyhow::Result,
        metadata::Metadata,
        signature::SignatureSerializer,
        value::hash::Hash,
    },
    pin::{Pinned, PinnedInner},
    tokio::io::{AsyncRead, AsyncReadExt},
};
use rkyv::{
    de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
    Deserialize, Serialize,
};

#[async_trait]
pub trait Ipiis {
    type Address: Send + Sync;

    fn account_me(&self) -> &Account;

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef>;

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()>;

    async fn get_address(&self, target: &AccountRef) -> Result<<Self as Ipiis>::Address>;

    async fn set_address(
        &self,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()>;

    fn sign<T>(&self, target: AccountRef, msg: T) -> Result<GuaranteeSigned<T>>
    where
        T: Archive + Serialize<SignatureSerializer> + Send,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Metadata::builder().build(self.account_me(), target, msg)
    }

    fn sign_as_guarantor<T>(&self, msg: GuaranteeSigned<T>) -> Result<GuarantorSigned<T>>
    where
        T: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq + Send,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Signer::sign(self.account_me(), msg)
    }

    async fn call<'res, Req, Res>(
        &self,
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

        // recv data
        let res = self.call_unchecked(target, msg).await?;

        // verify data
        let () = res.verify(Some(self.account_me().account_ref()))?;

        Ok(res)
    }

    async fn call_unchecked<'res, Req, Res>(
        &self,
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
        // send data
        let msg = ::ipis::rkyv::to_bytes::<_, SERIALIZER_HEAP_SIZE>(&msg)?;

        // recv data
        let bytes = self.call_raw_to_end(target, &mut msg.as_ref()).await?;

        // unpack data
        let res = PinnedInner::<GuaranteeSigned<Res>>::new(bytes)?;

        Ok(res)
    }

    async fn call_permanent<'res, Req, Res>(
        &self,
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
        // sign data
        let msg = self.sign(*target, msg)?;

        // recv data
        let res = self.call_unchecked(target, msg).await?;

        // verify data
        let () = res.verify(Some(self.account_me().account_ref()))?;

        Ok(res)
    }

    async fn call_permanent_unchecked<'res, Req, Res>(
        &self,
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
        // sign data
        let msg = self.sign(*target, msg)?;

        // recv data
        self.call_unchecked(target, msg).await
    }

    async fn call_deserialized<Req, Res>(
        &self,
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
        // recv data
        let res = self.call_deserialized_unchecked(target, msg).await?;

        // verify data
        let () = res.verify(Some(self.account_me().account_ref()))?;

        Ok(res)
    }

    async fn call_deserialized_unchecked<Req, Res>(
        &self,
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
        // recv data
        self.call_unchecked(target, msg)
            .await
            // unpack data
            .and_then(|e: Pinned<GuaranteeSigned<Res>>| e.deserialize_into())
    }

    async fn call_permanent_deserialized<Req, Res>(
        &self,
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
        // recv data
        let res: GuaranteeSigned<Res> = self
            .call_permanent_deserialized_unchecked(target, msg)
            .await?;

        // verify data
        let () = res.verify(Some(self.account_me().account_ref()))?;

        Ok(res)
    }

    async fn call_permanent_deserialized_unchecked<Req, Res>(
        &self,
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
        // sign data
        let msg = self.sign(*target, msg)?;

        // recv data
        self.call_deserialized_unchecked::<Req, Res>(target, msg)
            .await
    }

    async fn call_raw<Req>(
        &self,
        target: &AccountRef,
        msg: &mut Req,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>>
    where
        Req: AsyncRead + Send + Sync + Unpin;

    async fn call_raw_exact<Req>(
        &self,
        target: &AccountRef,
        msg: &mut Req,
        buf: &mut [u8],
    ) -> Result<usize>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        self.call_raw(target, msg)
            .await?
            .read_exact(buf)
            .await
            .map_err(Into::into)
    }

    async fn call_raw_to_end<Req>(&self, target: &AccountRef, msg: &mut Req) -> Result<Vec<u8>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        let mut recv = self.call_raw(target, msg).await?;

        // create a buffer
        let mut buf = {
            let len = recv.read_u64().await?;
            vec![0; len.try_into()?]
        };

        recv.read_exact(&mut buf).await?;
        Ok(buf)
    }
}

#[async_trait]
impl<Client, IpiisClient> Ipiis for Client
where
    Client: ::core::ops::Deref<Target = IpiisClient> + Send + Sync,
    IpiisClient: Ipiis + Send + Sync + 'static,
    <IpiisClient as Ipiis>::Address: 'static,
{
    type Address = <IpiisClient as Ipiis>::Address;

    fn account_me(&self) -> &Account {
        (**self).account_me()
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        (**self).get_account_primary(kind).await
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        (**self).set_account_primary(kind, account).await
    }

    async fn get_address(&self, target: &AccountRef) -> Result<<Self as Ipiis>::Address> {
        (**self).get_address(target).await
    }

    async fn set_address(
        &self,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        (**self).set_address(target, address).await
    }

    fn sign<T>(&self, target: AccountRef, msg: T) -> Result<GuaranteeSigned<T>>
    where
        T: Archive + Serialize<SignatureSerializer> + Send,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        (**self).sign(target, msg)
    }

    async fn call<'res, Req, Res>(
        &self,
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
        (**self).call(target, msg).await
    }

    async fn call_unchecked<'res, Req, Res>(
        &self,
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
        (**self).call_unchecked(target, msg).await
    }

    async fn call_permanent<'res, Req, Res>(
        &self,
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
        (**self).call_permanent(target, msg).await
    }

    async fn call_permanent_unchecked<'res, Req, Res>(
        &self,
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
        (**self).call_permanent_unchecked(target, msg).await
    }

    async fn call_deserialized<Req, Res>(
        &self,
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
        (**self).call_deserialized(target, msg).await
    }

    async fn call_deserialized_unchecked<Req, Res>(
        &self,
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
        (**self).call_deserialized_unchecked(target, msg).await
    }

    async fn call_permanent_deserialized<Req, Res>(
        &self,
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
        (**self).call_permanent_deserialized(target, msg).await
    }

    async fn call_permanent_deserialized_unchecked<Req, Res>(
        &self,
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
        (**self)
            .call_permanent_deserialized_unchecked(target, msg)
            .await
    }

    async fn call_raw<Req>(
        &self,
        target: &AccountRef,
        msg: &mut Req,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        (**self).call_raw(target, msg).await
    }

    async fn call_raw_exact<Req>(
        &self,
        target: &AccountRef,
        msg: &mut Req,
        buf: &mut [u8],
    ) -> Result<usize>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        (**self).call_raw_exact(target, msg, buf).await
    }

    async fn call_raw_to_end<Req>(&self, target: &AccountRef, msg: &mut Req) -> Result<Vec<u8>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        (**self).call_raw_to_end(target, msg).await
    }
}

pub type Request<Address> = GuaranteeSigned<RequestType<Address>>;

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[archive(bound(archive = "
    <Address as Archive>::Archived: ::core::fmt::Debug + PartialEq,
",))]
#[archive_attr(derive(CheckBytes, Debug, PartialEq))]
pub enum RequestType<Address> {
    GetAccountPrimary {
        kind: Option<Hash>,
    },
    SetAccountPrimary {
        kind: Option<Hash>,
        account: AccountRef,
    },
    GetAddress {
        account: AccountRef,
    },
    SetAddress {
        account: AccountRef,
        address: Address,
    },
}

#[derive(Clone, Debug, PartialEq, Archive, Serialize, Deserialize)]
#[archive(bound(archive = "
    <Address as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    <Option<Address> as Archive>::Archived: ::core::fmt::Debug + PartialEq,
",))]
#[archive_attr(derive(CheckBytes, Debug, PartialEq))]
pub enum Response<Address> {
    GetAccountPrimary {
        account: AccountRef,
        address: Option<Address>,
    },
    SetAccountPrimary,
    GetAddress {
        address: Address,
    },
    SetAddress,
}

pub type Serializer = ::ipis::rkyv::ser::serializers::AllocSerializer<SERIALIZER_HEAP_SIZE>;

pub const SERIALIZER_HEAP_SIZE: usize = 4096;

#[macro_export]
macro_rules! external_call {
    (
        call: $call:expr,
        response: $ty:ty => $kind:ident,
    ) => {
        external_call!(
            call: $call,
            response: $ty => $kind,
            items: {},
        )
    };
    (
        call: $call:expr,
        response: $ty:ty => $kind:ident,
        items: { $( $items:ident ),* },
    ) => {{
        let __res: ::ipis::core::account::GuaranteeSigned<$ty> = $call;

        // unpack response
        match __res.data.data {
            <$ty>::$kind { $( $items, )* .. } => ( $( $items, )* ),
            _ => ::ipis::core::anyhow::bail!("failed to parse response"),
        }
    }};
}
