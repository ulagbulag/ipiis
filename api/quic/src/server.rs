use std::{net::SocketAddr, sync::Arc, time::Duration};

use ipiis_api_common::impl_ipiis_server;
use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef},
        anyhow::{bail, Result},
    },
    env::{infer, Infer},
    futures::{Future, StreamExt},
    log::{error, info, warn},
    tokio::sync::Mutex,
};
use quinn::{Endpoint, Incoming, IncomingBiStreams, ServerConfig};

impl_ipiis_server!(client: crate::client::IpiisClient, server: IpiisServer,);

pub struct IpiisServer {
    pub(crate) client: crate::client::IpiisClient,
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
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_account_primary").ok();
        let account_port = infer("ipiis_server_port")?;

        Self::new(account_me, account_primary, account_port).await
    }

    async fn genesis(
        port: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        // generate an account
        let account = Account::generate();
        let account_primary = infer("ipiis_account_primary").ok();

        // init a server
        let server = Self::new(account, account_primary, port).await?;

        Ok(server)
    }
}

impl IpiisServer {
    pub async fn new(
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
                let (priv_key, cert_chain) = crate::cert::generate(&account_me)?;

                let mut config = ServerConfig::with_single_cert(cert_chain, priv_key)?;
                config.transport = {
                    let mut config = Arc::try_unwrap(config.transport).unwrap();
                    config.max_idle_timeout(Some(Duration::from_secs(10).try_into()?));
                    config.keep_alive_interval(Some(Duration::from_secs(5)));
                    config.into()
                };
                config
            };
            let addr = format!("0.0.0.0:{port}").parse()?;

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
            )
            .await?,
            incoming: Mutex::new(incoming),
        })
    }

    pub async fn run<C, F, Fut>(&self, client: Arc<C>, handler: F)
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        F: Fn(
                Arc<C>,
                <crate::client::IpiisClient as Ipiis>::Writer,
                <crate::client::IpiisClient as Ipiis>::Reader,
            ) -> Fut
            + Copy
            + Send
            + 'static,
        Fut: Future<Output = Result<()>> + Send,
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
                    info!("incoming connection: addr={addr}");

                    {
                        // Each stream initiated by the client constitutes a new request.
                        let client = client.clone();

                        ::ipis::tokio::spawn(async move {
                            Self::handle_connection(client, addr, bi_streams, handler).await
                        });
                    }
                }
                Err(e) => {
                    warn!("incoming connection error: {e}");
                }
            }
        }
    }

    async fn handle_connection<C, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        bi_streams: IncomingBiStreams,
        handler: F,
    ) where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        F: Fn(
                Arc<C>,
                <crate::client::IpiisClient as Ipiis>::Writer,
                <crate::client::IpiisClient as Ipiis>::Reader,
            ) -> Fut
            + Copy
            + Send
            + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        match Self::try_handle_connection(client, addr, bi_streams, handler).await {
            Ok(_) => (),
            Err(e) => warn!("handling error: addr={addr}, {e}"),
        }
    }

    async fn try_handle_connection<C, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        mut bi_streams: IncomingBiStreams,
        handler: F,
    ) -> Result<()>
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        F: Fn(
                Arc<C>,
                <crate::client::IpiisClient as Ipiis>::Writer,
                <crate::client::IpiisClient as Ipiis>::Reader,
            ) -> Fut
            + Copy
            + Send
            + 'static,
        Fut: Future<Output = Result<()>> + Send,
    {
        while let Some(stream) = bi_streams.next().await {
            match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    info!("connection closed: addr={addr}");
                    break;
                }
                Err(e) => {
                    bail!("connection error: {e}");
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

    async fn handle<C, F, Fut>(
        client: Arc<C>,
        addr: SocketAddr,
        stream: (
            <crate::client::IpiisClient as Ipiis>::Writer,
            <crate::client::IpiisClient as Ipiis>::Reader,
        ),
        handler: F,
    ) where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        F: Fn(
            Arc<C>,
            <crate::client::IpiisClient as Ipiis>::Writer,
            <crate::client::IpiisClient as Ipiis>::Reader,
        ) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        match Self::try_handle(client, stream, handler).await {
            Ok(_) => (),
            Err(e) => error!("error handling: addr={addr}, {e}"),
        }
    }

    fn try_handle<C, F, Fut>(
        client: Arc<C>,
        (send, recv): (
            <crate::client::IpiisClient as Ipiis>::Writer,
            <crate::client::IpiisClient as Ipiis>::Reader,
        ),
        handler: F,
    ) -> impl Future<Output = Result<()>>
    where
        C: AsRef<crate::client::IpiisClient> + Send + Sync + 'static,
        F: Fn(
            Arc<C>,
            <crate::client::IpiisClient as Ipiis>::Writer,
            <crate::client::IpiisClient as Ipiis>::Reader,
        ) -> Fut,
        Fut: Future<Output = Result<()>>,
    {
        // handle data
        handler(client, send, recv)
    }
}
