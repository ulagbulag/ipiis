use std::{net::SocketAddr, sync::Arc};

use ipiis_api_common::impl_ipiis_server;
use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef},
        anyhow::Result,
    },
    env::{infer, Infer},
    futures::Future,
    log::{error, info, warn},
    tokio,
};

impl_ipiis_server!(client: crate::client::IpiisClient, server: IpiisServer,);

pub struct IpiisServer {
    pub(crate) client: crate::client::IpiisClient,
    incoming: tokio::net::TcpListener,
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
        let incoming = {
            let addr: SocketAddr = format!("0.0.0.0:{port}").parse()?;

            let incoming = tokio::net::TcpListener::bind(addr).await?;

            incoming
        };

        Ok(Self {
            client: crate::client::IpiisClient::with_address_db_path(
                account_me,
                account_primary,
                "ipiis_server_address_db",
            )
            .await?,
            incoming,
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
        loop {
            match self.incoming.accept().await {
                Ok((stream, addr)) => {
                    info!("incoming connection: addr={addr}");

                    {
                        // Each stream initiated by the client constitutes a new request.
                        let client = client.clone();

                        let (recv, send) = tokio::io::split(stream);

                        ::ipis::tokio::spawn(async move {
                            Self::handle(client, addr, (send, recv), handler).await
                        });
                    }
                }
                Err(e) => {
                    warn!("incoming connection error: {e}");
                }
            }
        }
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
