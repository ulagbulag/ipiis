use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, GuarantorSigned},
        anyhow::Result,
        data::Data,
        signature::SignatureSerializer,
        signed::IsSigned,
        value::hash::Hash,
    },
    tokio::io::{AsyncRead, AsyncWrite},
};
use rkyv::{Archive, Serialize};

#[async_trait]
pub trait Ipiis {
    type Address: IsSigned + Send + Sync;
    type Reader: AsyncRead + Send + Unpin + 'static;
    type Writer: AsyncWrite + Send + Unpin + 'static;

    /// # Safety
    /// The source code itself is completely safe.
    /// However, if two or more keys exist at the same time by calling this function,
    /// some fatal security flaw such as key leakage may occur.
    /// So please be careful when using it.
    ///
    unsafe fn account_me(&self) -> Result<&Account>;

    fn account_ref(&self) -> &AccountRef;

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef>;

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()>;

    async fn get_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<<Self as Ipiis>::Address>;

    async fn set_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()>;

    fn sign<'a, T>(&self, target: AccountRef, msg: &'a T) -> Result<Data<GuaranteeSigned, &'a T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Data::builder().build(unsafe { self.account_me() }?, target, msg)
    }

    fn sign_owned<T>(&self, target: AccountRef, msg: T) -> Result<Data<GuaranteeSigned, T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        Data::builder().build_owned(unsafe { self.account_me() }?, target, msg)
    }

    fn sign_as_guarantor<T>(
        &self,
        msg: Data<GuaranteeSigned, T>,
    ) -> Result<Data<GuarantorSigned, T>>
    where
        T: IsSigned,
    {
        msg.sign(unsafe { self.account_me() }?)
    }

    fn protocol(&self) -> String;

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)>;
}

#[async_trait]
impl<Client, IpiisClient> Ipiis for Client
where
    Client: ::core::ops::Deref<Target = IpiisClient> + Send + Sync,
    IpiisClient: Ipiis + Send + Sync + 'static,
    <IpiisClient as Ipiis>::Address: 'static,
{
    type Address = <IpiisClient as Ipiis>::Address;
    type Reader = <IpiisClient as Ipiis>::Reader;
    type Writer = <IpiisClient as Ipiis>::Writer;

    unsafe fn account_me(&self) -> Result<&Account> {
        (**self).account_me()
    }

    fn account_ref(&self) -> &AccountRef {
        (**self).account_ref()
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        (**self).get_account_primary(kind).await
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        (**self).set_account_primary(kind, account).await
    }

    async fn get_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<<Self as Ipiis>::Address> {
        (**self).get_address(kind, target).await
    }

    async fn set_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        (**self).set_address(kind, target, address).await
    }

    fn sign<'a, T>(&self, target: AccountRef, msg: &'a T) -> Result<Data<GuaranteeSigned, &'a T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        (**self).sign(target, msg)
    }

    fn sign_owned<T>(&self, target: AccountRef, msg: T) -> Result<Data<GuaranteeSigned, T>>
    where
        T: Archive + Serialize<SignatureSerializer> + IsSigned,
        <T as Archive>::Archived: ::core::fmt::Debug + PartialEq,
    {
        (**self).sign_owned(target, msg)
    }

    fn sign_as_guarantor<T>(
        &self,
        msg: Data<GuaranteeSigned, T>,
    ) -> Result<Data<GuarantorSigned, T>>
    where
        T: IsSigned,
    {
        (**self).sign_as_guarantor(msg)
    }

    fn protocol(&self) -> String {
        (**self).protocol()
    }

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)> {
        (**self).call_raw(kind, target).await
    }
}

pub const CLIENT_DUMMY: u8 = 42;
::ipis::bitflags::bitflags! {

    pub struct ServerResult: u8 {
        const ACK = 0b10000000;
        const OK = 0b01000000;
        const ERR = 0b00100000;

        const ACK_OK = Self::ACK.bits | Self::OK.bits;
        const ACK_ERR = Self::ACK.bits | Self::ERR.bits;
    }
}

define_io! {
    GetAccountPrimary {
        inputs: { },
        input_sign: Data<GuaranteeSigned, Option<Hash>>,
        outputs: {
            account: AccountRef,
            address: Option<Address>,
        },
        output_sign: Data<GuarantorSigned, Option<Hash>>,
        generics: { Address, },
    },
    SetAccountPrimary {
        inputs: { },
        input_sign: Data<GuaranteeSigned, (Option<Hash>, AccountRef)>,
        outputs: { },
        output_sign: Data<GuarantorSigned, (Option<Hash>, AccountRef)>,
        generics: { },
    },
    GetAddress {
        inputs: { },
        input_sign: Data<GuaranteeSigned, (Option<Hash>, AccountRef)>,
        outputs: {
            address: Address,
        },
        output_sign: Data<GuarantorSigned, (Option<Hash>, AccountRef)>,
        generics: { Address, },
    },
    SetAddress {
        inputs: { },
        input_sign: Data<GuaranteeSigned, (Option<Hash>, AccountRef, Address)>,
        outputs: { },
        output_sign: Data<GuarantorSigned, (Option<Hash>, AccountRef, Address)>,
        generics: { Address, },
    },
}

#[macro_export]
macro_rules! define_io {
    (
        $($case:ident {
            inputs: { $( $input_field:ident : $input_ty:ty ,)* },
            input_sign: $input_sign:ty,
            outputs: { $( $output_field:ident : $output_ty:ty ,)* },
            output_sign: $output_sign:ty,
            generics: { $( $generic:ident ,)* },
        },)*
    ) => {::ipis::paste::paste! {
        pub mod io {
            use bytecheck::CheckBytes;
            use rkyv::{Archive, Deserialize, Serialize};

            #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
            #[archive(compare(PartialEq))]
            #[archive_attr(derive(CheckBytes, Copy, Clone, Debug, PartialEq, Eq, Hash))]
            pub enum OpCode {$(
                $case,
            )*}

            impl ::ipis::core::signed::IsSigned for OpCode {}

            pub mod request {
                use super::super::*;

                $(
                    pub struct $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub __lifetime: ::core::marker::PhantomData<&'__io ((), $( $generic, )* )>,
                        pub __sign: ::ipis::stream::DynStream<'__io, $input_sign>,
                        $(
                            pub $input_field: ::ipis::stream::DynStream<'__io, $input_ty>,
                        )*
                    }

                    impl<'__io, $( $generic, )* > ::ipis::core::signed::IsSigned for $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                    }

                    impl<'__io, $( $generic, )* > $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub async fn call<__IpiisClient>(
                            &'__io mut self,
                            client: &__IpiisClient,
                            kind: Option<&::ipis::core::value::hash::Hash>,
                            target: &::ipis::core::account::AccountRef,
                        ) -> ::ipis::core::anyhow::Result<super::response::$case<'static, $( $generic, )* >>
                        where
                            __IpiisClient: super::super::Ipiis,
                            <::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String> as ::ipis::rkyv::Archive>::Archived: ::ipis::rkyv::Deserialize<
                                    ::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String>,
                                    ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                >,
                            $(
                                $input_ty: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + Send
                                    + Sync
                                    + 'static,
                                <$input_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $input_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                                )*
                            $(
                                $output_ty: ::ipis::rkyv::Archive + ::core::fmt::Debug + PartialEq + 'static,
                                <$output_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $output_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                            $(
                                $generic: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + ::core::fmt::Debug
                                    + PartialEq
                                    + Send
                                    + Sync
                                    + 'static,
                                <$generic as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $generic,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                        {
                            // send data
                            let recv = self.send(client, kind, target).await?;

                            // recv data
                            super::response::$case::recv(target, recv).await
                        }

                        pub async fn send<__IpiisClient>(
                            &'__io mut self,
                            client: &__IpiisClient,
                            kind: Option<&::ipis::core::value::hash::Hash>,
                            target: &::ipis::core::account::AccountRef,
                        ) -> ::ipis::core::anyhow::Result<<__IpiisClient as super::super::Ipiis>::Reader>
                        where
                            __IpiisClient: super::super::Ipiis,
                            <::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String> as ::ipis::rkyv::Archive>::Archived: ::ipis::rkyv::Deserialize<
                                    ::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String>,
                                    ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                >,
                            $(
                                $input_ty: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + Send
                                    + Sync
                                    + 'static,
                                <$input_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $input_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                                )*
                            $(
                                $generic: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + ::core::fmt::Debug
                                    + PartialEq
                                    + Send
                                    + Sync
                                    + 'static,
                                <$generic as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $generic,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                        {
                            use ipis::tokio::io::AsyncReadExt;

                            // make a opcode
                            let mut opcode = ::ipis::stream::DynStream::Owned(super::OpCode::$case);

                            // pack data
                            opcode.serialize_inner().await?;
                            self.__sign.serialize_inner().await?;
                            $(
                                {
                                    self.$input_field.serialize_inner().await?;
                                }
                            )*

                            // make a connection
                            let (mut send, mut recv) = client.call_raw(kind, target).await?;

                            // send opcode
                            opcode.copy_to(&mut send).await?;

                            // send sign
                            self.__sign.copy_to(&mut send).await?;

                            // send data
                            $(
                                {
                                    self.$input_field.copy_to(&mut send).await?;
                                }
                            )*

                            // recv flag
                            match recv.read_u8().await.map(super::super::ServerResult::from_bits) {
                                // parse the data
                                Ok(Some(super::super::ServerResult::ACK_OK)) => Ok(recv),
                                // parse the error
                                Ok(Some(super::super::ServerResult::ACK_ERR)) => {
                                    // recv data
                                    let res: String = ::ipis::stream::DynStream::recv(&mut recv)
                                        .await?
                                        .to_owned().await?;

                                    // TODO: verify data

                                    ::ipis::core::anyhow::bail!("internal error: {res}")
                                }
                                Ok(Some(flag)) if flag.contains(super::super::ServerResult::ACK) => {
                                    ::ipis::core::anyhow::bail!("unknown ACK flag: {flag:?}")
                                }
                                Ok(Some(_) | None) => {
                                    ::ipis::core::anyhow::bail!("cannot parse the result of response")
                                }
                                Err(e) => {
                                    ::ipis::core::anyhow::bail!("network error: {e}")
                                }
                            }
                        }
                    }

                    impl<$( $generic, )* > $case<'static, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub async fn recv<__IpiisClient>(
                            client: &__IpiisClient,
                            mut recv: impl ::ipis::tokio::io::AsyncRead + Unpin,
                        ) -> ::ipis::core::anyhow::Result<Self>
                        where
                            __IpiisClient: super::super::Ipiis,
                            <::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String> as ::ipis::rkyv::Archive>::Archived: ::ipis::rkyv::Deserialize<
                                    ::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String>,
                                    ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                >,
                            $(
                                $input_ty: ::ipis::rkyv::Archive + ::core::fmt::Debug + PartialEq + 'static,
                                <$input_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $input_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                            $(
                                $generic: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + ::core::fmt::Debug
                                    + PartialEq
                                    + Send
                                    + Sync
                                    + 'static,
                                <$generic as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $generic,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                        {
                            use ipis::core::account::Verifier;

                            // recv data
                            let mut res = Self {
                                __lifetime: Default::default(),
                                __sign: ::ipis::stream::DynStream::recv(&mut recv).await?,
                                $(
                                    $input_field: ::ipis::stream::DynStream::recv(&mut recv).await?,
                                )*
                            };

                            // verify data
                            {
                                // select the sign data
                                let data = res.__sign.as_ref().await?;

                                // verify it
                                data.verify(Some(client.account_ref()))?
                            };

                            Ok(res)
                        }
                    }
                )*
            }

            pub mod response {
                use super::super::*;

                $(
                    pub struct $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub __lifetime: ::core::marker::PhantomData<&'__io ((), $( $generic, )* )>,
                        pub __sign: ::ipis::stream::DynStream<'__io, $output_sign>,
                        $(
                            pub $output_field: ::ipis::stream::DynStream<'__io, $output_ty>,
                        )*
                    }

                    impl<'__io, $( $generic, )* > ::ipis::core::signed::IsSigned for $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                    }

                    impl<'__io, $( $generic, )* > $case<'__io, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub async fn send<__IpiisClient>(
                            &'__io mut self,
                            _client: &__IpiisClient,
                            mut send: &mut <__IpiisClient as super::super::Ipiis>::Writer,
                        ) -> ::ipis::core::anyhow::Result<()>
                        where
                            __IpiisClient: super::super::Ipiis,
                            <::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String> as ::ipis::rkyv::Archive>::Archived: ::ipis::rkyv::Deserialize<
                                    ::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String>,
                                    ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                >,
                            $(
                                $output_ty: ::ipis::rkyv::Archive + ::core::fmt::Debug + PartialEq + 'static,
                                <$output_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $output_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                            $(
                                $generic: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + ::core::fmt::Debug
                                    + PartialEq
                                    + Send
                                    + Sync
                                    + 'static,
                                <$generic as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $generic,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                        {
                            use ipis::tokio::io::AsyncWriteExt;

                            // make a flag
                            let flag = super::super::ServerResult::ACK_OK;

                            // send flag
                            send.write_u8(flag.bits()).await?;

                            // send sign
                            self.__sign.copy_to(&mut send).await?;

                            // send data
                            $(
                                {
                                    self.$output_field.copy_to(&mut send).await?;
                                }
                            )*
                            Ok(())
                        }
                    }

                    impl<$( $generic, )* > $case<'static, $( $generic, )* >
                    where
                        $(
                            $generic: ::rkyv::Archive + Clone + ::core::fmt::Debug + PartialEq + ::ipis::core::signed::IsSigned,
                            <$generic as ::rkyv::Archive>::Archived: ::core::fmt::Debug + PartialEq,
                        )*
                    {
                        pub async fn recv(
                            target: &::ipis::core::account::AccountRef,
                            mut recv: impl ::ipis::tokio::io::AsyncRead + Unpin,
                        ) -> ::ipis::core::anyhow::Result<Self>
                        where
                            <::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String> as ::ipis::rkyv::Archive>::Archived: ::ipis::rkyv::Deserialize<
                                    ::ipis::core::data::Data<::ipis::core::account::GuaranteeSigned, String>,
                                    ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                >,
                            $(
                                $output_ty: ::ipis::rkyv::Archive + ::core::fmt::Debug + PartialEq + 'static,
                                <$output_ty as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $output_ty,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                            $(
                                $generic: ::ipis::core::signed::IsSigned
                                    + ::ipis::rkyv::Archive
                                    + ::ipis::rkyv::Serialize<::ipis::core::signature::SignatureSerializer>
                                    + ::ipis::rkyv::Serialize<::ipis::core::signed::Serializer>
                                    + ::core::fmt::Debug
                                    + PartialEq
                                    + Send
                                    + Sync
                                    + 'static,
                                <$generic as ::ipis::rkyv::Archive>::Archived: for<'__bytecheck> ::ipis::bytecheck::CheckBytes<
                                        ::ipis::rkyv::validation::validators::DefaultValidator<'__bytecheck>,
                                    > + ::ipis::rkyv::Deserialize<
                                        $generic,
                                        ::ipis::rkyv::de::deserializers::SharedDeserializeMap,
                                    >
                                    + ::core::fmt::Debug
                                    + PartialEq,
                            )*
                        {
                            use ipis::core::account::Verifier;

                            // recv data
                            let mut res = Self {
                                __lifetime: Default::default(),
                                __sign: ::ipis::stream::DynStream::recv(&mut recv).await?,
                                $(
                                    $output_field: ::ipis::stream::DynStream::recv(&mut recv).await?,
                                )*
                            };

                            // verify data
                            {
                                // select the sign data
                                let data = res.__sign.as_ref().await?;

                                // verify it
                                data.verify(Some(target))?
                            };

                            Ok(res)
                        }
                    }
                )*
            }
        }
    }};
}

/// # External Call
///
/// ## Usage
///
/// ```ignore
/// // external call
/// let (address,): (Option<::std::net::SocketAddr>,) = external_call!(
///     client: self,
///     target: None => &primary,
///     request: ::ipiis_common::io => GetAccountPrimary,
///     sign: self.sign(primary, Some(*kind))?,
///     inputs: {
///         sign: self.sign(primary, Some(*kind))?,
///         kind: Some(*kind),
///     },
///     outputs: { account, address, },
/// );
/// ```
///
#[macro_export]
macro_rules! external_call {
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        $( inputs_mode: $mode:ident ,)?
    ) => {
        external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : $input_value ,)* },
            $( inputs_mode: $mode ,)?
            outputs: { },
        )
    };
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        $( inputs_mode: $mode:ident ,)?
        outputs: { $( $output:ident ,)* },
    ) => {{
        use ipis::core::signed::IsSigned;

        // external call
        #[allow(clippy::redundant_field_names)]
        let mut res = external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : $input_value ,)* },
            $( inputs_mode: $mode ,)?
            outputs: call,
        );

        // unpack response
        #[allow(clippy::unused_unit)]
        {( $( res.$output.to_owned().await?, )* )}
    }};
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        $( inputs_mode: $mode:ident ,)?
        outputs: call,
    ) => {{
        // pack request
        #[allow(clippy::redundant_field_names)]
        let mut req = external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : $input_value ,)* },
            $( inputs_mode: $mode ,)?
            outputs: none,
        );

        // recv response
        req.call($client, $kind, $target).await?
    }};
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        $( inputs_mode: $mode:ident ,)?
        outputs: send,
    ) => {{
        // pack request
        #[allow(clippy::redundant_field_names)]
        let mut req = external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : $input_value ,)* },
            $( inputs_mode: $mode ,)?
            outputs: none,
        );

        // recv response
        req.send($client, $kind, $target).await?
    }};
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        outputs: none,
    ) => {{
        external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : $input_value ,)* },
            inputs_mode: owned,
            outputs: none,
        )
    }};
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        inputs_mode: owned,
        outputs: none,
    ) => {{
        external_call!(
            client: $client,
            target: $kind => $target,
            request: $io => $req,
            sign: $input_sign,
            inputs: { $( $input_field : ::ipis::stream::DynStream::Owned($input_value) ,)* },
            inputs_mode: none,
            outputs: none,
        )
    }};
    (
        client: $client:expr,
        target: $kind:expr => $target:expr,
        request: $io:path => $req:ident,
        sign: $input_sign:expr,
        inputs: { $( $input_field:ident : $input_value:expr ,)* },
        inputs_mode: none,
        outputs: none,
    ) => {{
        use ipis::core::signed::IsSigned;

        use $io::{request::$req};

        // sign data
        let mut sign: ::ipis::stream::DynStream<_> = {
            // select the sign data
            let data = $input_sign;

            // sign it
            if data.is_signed_dyn() {
                ::ipis::stream::DynStream::Owned(data)
            } else {
                ::ipis::stream::DynStream::OwnedAlignedVec($client.sign_owned(*$target, data)?.to_bytes()?)
            }
        };

        // pack request
        #[allow(clippy::redundant_field_names)]
        $req {
            __lifetime: Default::default(),
            __sign: sign,
            $( $input_field: $input_value ,)*
        }
    }};
}

/// # Handling External Call
///
/// ## Usage
///
/// ```ignore
/// handle_external_call!(
///      server: IpiisServer,
///      name: run_ipiis,
///      request: ::ipiis_common::io => {
///          GetAccountPrimary => handle_get_account_primary,
///          SetAccountPrimary => handle_set_account_primary,
///          GetAddress => handle_get_address,
///          SetAddress => handle_set_address,
///      },
///  );
/// ```
///
#[macro_export]
macro_rules! handle_external_call {
    (
        server: $server:ty => $client:ty,
        name: $name:ident,
        request: $io:path => { $( $opcode:ident => $handler:ident ,)* },
        $( request_raw: $io_raw:path => { $( $opcode_raw:ident => $handler_raw:ident ,)* },)?
    ) => {
        impl $server {
            pub async fn $name(self) {
                let client = self.client.clone();

                let runtime: &IpiisServer = (*self.client).as_ref();
                runtime.run(client, Self::__handle::<IpiisClient>).await
            }
        }

        handle_external_call!(
            server: $server => $client,
            request: $io => { $( $opcode => $handler ,)* },
            $( request_raw: $io_raw => { $( $opcode_raw => $handler_raw ,)* },)?
        );
    };
    (
        server: $server:ty => $client:ty,
        request: $io:path => { $( $opcode:ident => $handler:ident ,)* },
        $( request_raw: $io_raw:path => { $( $opcode_raw:ident => $handler_raw:ident ,)* },)?
    ) => {
        impl $server {
            async fn __handle<__IpiisClient>(
                client: Arc<$client>,
                mut send: <__IpiisClient as Ipiis>::Writer,
                mut recv: <__IpiisClient as Ipiis>::Reader,
            ) -> Result<()>
            where
                $client: AsRef<__IpiisClient>,
                __IpiisClient: Ipiis,
            {
                use ipis::tokio::io::AsyncWriteExt;

                match Self::__try_handle(&client, &mut send, recv).await {
                    Ok(()) => Ok(()),
                    Err(e) => {
                        // collect data
                        let mut data = ::ipis::stream::DynStream::Owned(e.to_string());

                        // make a flag
                        let flag = ServerResult::ACK_ERR;

                        // send flag
                        send.write_u8(flag.bits()).await?;

                        // send data
                        data.copy_to(&mut send).await?;

                        Ok(())
                    }
                }
            }

            async fn __try_handle<__IpiisClient>(
                client: &$client,
                send: &mut <__IpiisClient as Ipiis>::Writer,
                mut recv: <__IpiisClient as Ipiis>::Reader,
            ) -> Result<()>
            where
                $client: AsRef<__IpiisClient>,
                __IpiisClient: Ipiis,
            {
                use $io::{OpCode, request};

                // recv opcode
                let opcode: OpCode = ::ipis::stream::DynStream::recv(&mut recv)
                    .await?
                    .to_owned()
                    .await?;

                // select command
                match opcode {
                    $(
                        OpCode::$opcode => {
                            // recv request
                            let mut req = request::$opcode::recv(client.as_ref(), recv).await?;

                            // handle request
                            let mut res = Self::$handler(client, req).await?;

                            // send response
                            res.send(client.as_ref(), &mut *send).await
                        }
                    )*
                    $($(
                        OpCode::$opcode_raw => {
                            // handle raw request
                            let mut res = Self::$handler_raw(client, recv).await?;

                            // send response
                            res.send(client.as_ref(), &mut *send).await
                        },
                    )*)?
                }
            }
        }
    };
}
