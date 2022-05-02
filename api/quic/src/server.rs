use std::convert::Infallible;

use futures::StreamExt;
use ipis::core::{
    account::{Account, AccountRef},
    anyhow::Result,
};
use quinn::{Endpoint, ServerConfig};
use rustls::Certificate;

use crate::client::IpiisClient;

pub struct IpiisServer {
    // TODO: remove this struct, rather implement `listen(port) -> Result<!>` directly
    client: IpiisClient,
    port: u16,
}

impl IpiisServer {
    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
        port: u16,
    ) -> Result<Self> {
        Ok(Self {
            client: IpiisClient::with_address_db_path(
                account_me,
                account_primary,
                certs,
                "ipiis_server_address_db",
            )?,
            port,
        })
    }

    pub fn get_cert_chain(&self) -> Result<Vec<Certificate>> {
        crate::cert::generate(&self.client.account_me).map(|(_, e)| e)
    }

    fn get_server_config(&self) -> Result<ServerConfig> {
        let (priv_key, cert_chain) = crate::cert::generate(&self.client.account_me)?;

        ServerConfig::with_single_cert(cert_chain, priv_key).map_err(Into::into)
    }

    pub async fn run(self) -> Result<Infallible> {
        let server_config = self.get_server_config()?;
        let addr = format!("0.0.0.0:{}", self.port).parse()?;

        let (endpoint, mut incoming) = Endpoint::server(server_config, addr)?;

        loop {
            let quinn::NewConnection {
                connection: conn, ..
            } = incoming.next().await.unwrap().await.unwrap();
            println!(
                "[server] incoming connection: addr={}",
                conn.remote_address(),
            );
        }
    }
}
