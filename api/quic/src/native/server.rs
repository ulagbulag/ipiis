use core::{convert::Infallible, pin::Pin};

use futures::{Future, StreamExt};
use ipiis_common::{Ipiis, Serializer, SERIALIZER_HEAP_SIZE};
use ipis::{
    async_trait::async_trait,
    bytecheck::CheckBytes,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, Verifier},
        anyhow::{anyhow, Result},
        metadata::Metadata,
        signature::SignatureSerializer,
        value::chrono::DateTime,
    },
    env::{infer, Infer},
    log::error,
    pin::{Pinned, PinnedInner},
    rkyv,
    tokio::{
        io::{AsyncRead, AsyncReadExt},
        sync::Mutex,
    },
};
use quinn::{Endpoint, Incoming, ServerConfig};
use rkyv::{
    de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
    Deserialize, Serialize,
};
use rustls::Certificate;

use crate::common::{
    arp::{ArpRequest, ArpResponse},
    cert,
    opcode::Opcode,
};

pub struct IpiisServer {
    client: crate::client::IpiisClient,
    incoming: Mutex<Incoming>,
}

impl ::core::ops::Deref for IpiisServer {
    type Target = crate::client::IpiisClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[async_trait]
impl<'a> Infer<'a> for IpiisServer {
    type GenesisArgs = u16;
    type GenesisResult = (Self, Vec<Certificate>);

    fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_server_account_primary").ok();
        let certs = ::rustls_native_certs::load_native_certs()?
            .into_iter()
            .map(|e| Certificate(e.0))
            .collect::<Vec<_>>();
        let account_port = infer("ipiis_server_port")?;

        Self::new(account_me, account_primary, &certs, account_port)
    }

    fn genesis(
        port: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        // generate an account
        let account = Account::generate();

        // init a server
        let server = Self::new(account, None, &[], port)?;
        let certs = server.get_cert_chain()?;

        Ok((server, certs))
    }
}

impl IpiisServer {
    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
        port: u16,
    ) -> Result<Self> {
        let (endpoint, incoming) = {
            let mut cert_store = ::rustls::RootCertStore::empty();
            for cert in certs {
                cert_store.add(cert)?;
            }

            let client_config = ::quinn::ClientConfig::with_root_certificates(cert_store);
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

    pub fn get_cert_chain(&self) -> Result<Vec<Certificate>> {
        cert::generate(&self.client.account_me).map(|(_, e)| e)
    }

    pub async fn run<Req, Res, F, Fut>(&self, handler: F)
    where
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Pinned<GuaranteeSigned<Req>>) -> Fut,
        Fut: Future<Output = Result<Res>>,
    {
        match self.try_run(handler).await {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        }
    }

    pub async fn try_run<Req, Res, F, Fut>(&self, handler: F) -> Result<Infallible>
    where
        Req: Archive + Serialize<SignatureSerializer> + ::core::fmt::Debug + PartialEq,
        <Req as Archive>::Archived:
            for<'a> CheckBytes<DefaultValidator<'a>> + ::core::fmt::Debug + PartialEq,
        Res: Archive
            + Serialize<SignatureSerializer>
            + Serialize<Serializer>
            + ::core::fmt::Debug
            + PartialEq,
        <Res as Archive>::Archived: ::core::fmt::Debug + PartialEq,
        F: Fn(Pinned<GuaranteeSigned<Req>>) -> Fut,
        Fut: Future<Output = Result<Res>>,
    {
        let mut incoming = self.incoming.lock().await;

        loop {
            let quinn::NewConnection {
                connection: conn,
                mut bi_streams,
                ..
            } = incoming.next().await.unwrap().await.unwrap();
            println!(
                "[server] incoming connection: addr={}",
                conn.remote_address(),
            );

            // Each stream initiated by the client constitutes a new request.
            'stream: while let Some(stream) = bi_streams.next().await {
                let (mut send, mut recv) = match stream {
                    Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                        dbg!("connection closed");
                        break 'stream;
                    }
                    Err(e) => {
                        error!("connection error: {}", e);
                        break 'stream;
                    }
                    Ok(s) => s,
                };

                // recv opcode
                let opcode = recv.read_u8().await?;
                let buf = match Opcode::from_bits(opcode) {
                    Some(Opcode::ARP) => {
                        // recv data
                        let req = recv.read_to_end(usize::MAX).await?;

                        // unpack data
                        let req =
                            ::ipis::rkyv::check_archived_root::<GuaranteeSigned<ArpRequest>>(&req)
                                .map_err(|_| anyhow!("failed to parse the received bytes"))?;
                        let req: GuaranteeSigned<ArpRequest> =
                            req.deserialize(&mut SharedDeserializeMap::default())?;

                        // verify data
                        let () = req.verify(Some(self.client.account_me.account_ref()))?;

                        // handle data
                        let res = ArpResponse {
                            addr: self.client.get_address(&req.data.data.target).await?,
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
                        let () = req.verify(Some(self.client.account_me.account_ref()))?;

                        // handle data
                        let res = handler(req).await?;

                        // sign data
                        let res = {
                            let mut builder = Metadata::builder();

                            if let Some(expiration_date) = expiration_date {
                                builder = builder.expiration_date(expiration_date);
                            }

                            builder.build(&self.client.account_me, guarantee, res)?
                        };

                        // pack data
                        ::ipis::rkyv::to_bytes::<_, SERIALIZER_HEAP_SIZE>(&res)?
                    }
                    //
                    _ => {
                        error!("unknown opcode: {:x}", opcode);
                        break 'stream;
                    }
                };

                // send response
                send.write_all(&buf).await?;
                send.finish().await?;
            }
        }
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
