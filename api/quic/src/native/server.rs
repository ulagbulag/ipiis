use core::pin::Pin;
use std::{net::SocketAddr, sync::Arc};

use futures::{Future, StreamExt};
use ipiis_common::{Ipiis, Serializer, SERIALIZER_HEAP_SIZE};
use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, Verifier},
        anyhow::{anyhow, bail, Result},
        metadata::Metadata,
        signature::SignatureSerializer,
        value::chrono::DateTime,
    },
    env::{infer, Infer},
    log::{error, info, warn},
    pin::{Pinned, PinnedInner},
    rkyv,
    tokio::{
        io::{AsyncRead, AsyncReadExt},
        sync::Mutex,
    },
};
use quinn::{Endpoint, Incoming, IncomingBiStreams, ServerConfig};
use rkyv::{
    de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
    Deserialize, Serialize,
};

use crate::common::{
    arp::{ArpRequest, ArpResponse},
    cert,
    opcode::Opcode,
};

pub struct IpiisServer {
    client: crate::client::IpiisClient,
    incoming: Mutex<Incoming>,
}

impl AsRef<crate::client::IpiisClient> for IpiisServer {
    fn as_ref(&self) -> &crate::client::IpiisClient {
        &self.client
    }
}

#[async_trait]
impl<'a> Infer<'a> for IpiisServer {
    type GenesisArgs = u16;
    type GenesisResult = Self;

    fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_server_account_primary").ok();
        let account_port = infer("ipiis_server_port")?;

        Self::new(account_me, account_primary, account_port)
    }

    fn genesis(
        port: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        // generate an account
        let account = Account::generate();

        // init a server
        let server = Self::new(account, None, port)?;

        Ok(server)
    }
}

impl IpiisServer {
    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        port: u16,
    ) -> Result<Self> {
        let (endpoint, incoming) = {
            let crypto = ::rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_custom_certificate_verifier(super::cert::ServerVerification::new())
                .with_no_client_auth();
            let client_config = ::quinn::ClientConfig::new(Arc::new(crypto));

            let server_config = {
                let (priv_key, cert_chain) = cert::generate(&account_me)?;

                ServerConfig::with_single_cert(cert_chain, priv_key)?
            };
            let addr = format!("0.0.0.0:{}", port).parse()?;

            let (mut endpoint, incoming) = Endpoint::server(server_config, addr)?;
            endpoint.set_default_client_config(client_config);

            (endpoint, incoming)
        };

        Ok(Self {
            client: crate::client::IpiisClient::with_address_db_path(
                account_me,
                account_primary,
                "ipiis_server_address_db",
                endpoint,
            )?,
            incoming: Mutex::new(incoming),
        })
    }

    pub async fn run<C, Req, Res, F, Fut>(&self, client: Arc<C>, handler: F)
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Arc<C>, Pinned<GuaranteeSigned<Req>>) -> Fut + Copy + Send + Sync + 'static,
        Fut: Future<Output = Result<Res>> + Send,
    {
        let mut incoming = self.incoming.lock().await;

        while let Some(connection) = incoming.next().await {
            match connection.await {
                Ok(quinn::NewConnection {
                    connection: conn,
                    bi_streams,
                    ..
                }) => {
                    let addr = conn.remote_address();
                    info!("incoming connection: addr={}", addr);

                    {
                        // Each stream initiated by the client constitutes a new request.
                        let client = client.clone();

                        ::ipis::tokio::spawn(async move {
                            Self::handle_connection(client, addr, bi_streams, handler).await
                        });
                    }
                }
                Err(e) => {
                    warn!("incoming connection error: {}", e);
                }
            }
        }
    }

    async fn handle_connection<C, Req, Res, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        bi_streams: IncomingBiStreams,
        handler: F,
    ) where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Arc<C>, Pinned<GuaranteeSigned<Req>>) -> Fut + Copy + Send + Sync + 'static,
        Fut: Future<Output = Result<Res>> + Send,
    {
        match Self::try_handle_connection(client, addr, bi_streams, handler).await {
            Ok(_) => (),
            Err(e) => warn!("handling error: addr={}, {}", addr, e),
        }
    }

    async fn try_handle_connection<C, Req, Res, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        mut bi_streams: IncomingBiStreams,
        handler: F,
    ) -> Result<()>
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Arc<C>, Pinned<GuaranteeSigned<Req>>) -> Fut + Copy + Send + Sync + 'static,
        Fut: Future<Output = Result<Res>> + Send,
    {
        while let Some(stream) = bi_streams.next().await {
            match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed: addr={}", addr);
                    break;
                }
                Err(e) => {
                    bail!("connection error: {}", e);
                }
                Ok(stream) => {
                    let client = client.clone();

                    ::ipis::tokio::spawn(async move {
                        Self::handle(client, addr, stream, handler).await
                    });
                }
            }
        }
        Ok(())
    }

    async fn handle<C, Req, Res, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        stream: (quinn::SendStream, quinn::RecvStream),
        handler: F,
    ) where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Arc<C>, Pinned<GuaranteeSigned<Req>>) -> Fut,
        Fut: Future<Output = Result<Res>>,
    {
        match Self::try_handle(client, stream, handler).await {
            Ok(_) => (),
            Err(e) => error!("error handling: addr={}, {}", addr, e),
        }
    }

    async fn try_handle<C, Req, Res, F, Fut>(
        client: Arc<C>,
        (mut send, mut recv): (::quinn::SendStream, ::quinn::RecvStream),
        handler: F,
    ) -> Result<()>
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Arc<C>, Pinned<GuaranteeSigned<Req>>) -> Fut,
        Fut: Future<Output = Result<Res>>,
    {
        let ipiis_client: &crate::client::IpiisClient = client.as_ref().as_ref();
        let account_me = ipiis_client.account_me();
        let account_ref = account_me.account_ref();

        // recv opcode
        let opcode = recv.read_u8().await?;
        let buf = match Opcode::from_bits(opcode) {
            Some(Opcode::ARP) => {
                // recv data
                let req = recv.read_to_end(usize::MAX).await?;

                // unpack data
                let req = ::ipis::rkyv::check_archived_root::<GuaranteeSigned<ArpRequest>>(&req)
                    .map_err(|_| anyhow!("failed to parse the received bytes"))?;
                let req: GuaranteeSigned<ArpRequest> =
                    req.deserialize(&mut SharedDeserializeMap::default())?;

                // verify data
                let () = req.verify(Some(account_ref))?;

                // handle data
                let res = ArpResponse {
                    addr: ipiis_client.get_address(&req.data.data.target).await?,
                };

                // pack data
                ::ipis::rkyv::to_bytes::<_, SERIALIZER_HEAP_SIZE>(&res)?
            }
            Some(Opcode::TEXT) => {
                // recv data
                let req = recv.read_to_end(usize::MAX).await?;

                // unpack data
                let req = PinnedInner::<GuaranteeSigned<Req>>::new(req)?;
                let guarantee: AccountRef = req
                    .guarantee
                    .account
                    .deserialize(&mut SharedDeserializeMap::default())?;
                let expiration_date: Option<DateTime> = req
                    .data
                    .expiration_date
                    .deserialize(&mut SharedDeserializeMap::default())?;

                // verify data
                let () = req.verify(Some(account_ref))?;

                // handle data
                let res = handler(client.clone(), req).await?;

                // sign data
                let res = {
                    let mut builder = Metadata::builder();

                    if let Some(expiration_date) = expiration_date {
                        builder = builder.expiration_date(expiration_date);
                    }

                    builder.build(account_me, guarantee, res)?
                };

                // pack data
                ::ipis::rkyv::to_bytes::<_, SERIALIZER_HEAP_SIZE>(&res)?
            }
            //
            _ => {
                bail!("unknown opcode: {:x}", opcode);
            }
        };

        // send response
        send.write_all(&buf).await?;
        send.finish().await?;
        Ok(())
    }
}

#[async_trait]
impl Ipiis for IpiisServer {
    type Opcode = Opcode;

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
