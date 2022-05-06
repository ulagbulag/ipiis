use core::convert::Infallible;

use futures::{Future, StreamExt};
use ipiis_common::{Serializer, SERIALIZER_HEAP_SIZE};
use ipis::{
    bytecheck::CheckBytes,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, Verifier},
        anyhow::{anyhow, Result},
        metadata::Metadata,
        signature::SignatureSerializer,
        value::chrono::DateTime,
    },
    env::infer,
    log::error,
    pin::{Pinned, PinnedInner},
    rkyv,
    tokio::io::AsyncReadExt,
};
use quinn::{Endpoint, ServerConfig};
use rkyv::{
    de::deserializers::SharedDeserializeMap, validation::validators::DefaultValidator, Archive,
    Deserialize, Serialize,
};
use rustls::Certificate;

use crate::{
    client::IpiisClient,
    common::{
        arp::{ArpRequest, ArpResponse},
        cert,
        opcode::Opcode,
    },
};

pub struct IpiisServer {
    // TODO: remove this struct, rather implement `listen(port) -> Result<!>` directly
    client: crate::client::IpiisClient,
    port: u16,
}

impl ::core::ops::Deref for IpiisServer {
    type Target = crate::client::IpiisClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl IpiisServer {
    pub fn infer() -> Result<Self> {
        Ok(Self {
            client: IpiisClient::infer()?,
            port: infer("ipiis_server_port")?,
        })
    }

    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
        port: u16,
    ) -> Result<Self> {
        Ok(Self {
            client: crate::client::IpiisClient::with_address_db_path(
                account_me,
                account_primary,
                certs,
                "ipiis_server_address_db",
            )?,
            port,
        })
    }

    pub fn get_cert_chain(&self) -> Result<Vec<Certificate>> {
        cert::generate(&self.client.account_me).map(|(_, e)| e)
    }

    fn get_server_config(&self) -> Result<ServerConfig> {
        let (priv_key, cert_chain) = cert::generate(&self.client.account_me)?;

        ServerConfig::with_single_cert(cert_chain, priv_key).map_err(Into::into)
    }

    pub async fn run<Req, Res, F, Fut>(self, handler: F) -> Result<Infallible>
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
        let config = self.get_server_config()?;
        let addr = format!("0.0.0.0:{}", self.port).parse()?;

        let (_endpoint, mut incoming) = Endpoint::server(config, addr)?;

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
