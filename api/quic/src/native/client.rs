use core::pin::Pin;
use std::net::SocketAddr;

use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef},
        anyhow::{anyhow, bail, Result},
    },
    tokio::io::{AsyncRead, AsyncWriteExt},
};
use quinn::{Connection, Endpoint};
use rustls::Certificate;

use crate::common::{
    arp::{ArpRequest, ArpResponse},
    cert,
    opcode::Opcode,
};

pub struct IpiisClient {
    pub(crate) account_me: Account,
    account_primary: Option<AccountRef>,

    address_db: sled::Db,
    endpoint: Endpoint,
}

impl IpiisClient {
    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
    ) -> Result<Self> {
        Self::with_address_db_path(
            account_me,
            account_primary,
            certs,
            "ipiis_client_address_db",
        )
    }

    pub(crate) fn with_address_db_path<P>(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
        path: P,
    ) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        Ok(Self {
            account_me,
            account_primary,
            // TODO: allow to store in specific directory
            address_db: sled::open(tempfile::tempdir()?.path().join(path))?,
            endpoint: {
                let mut cert_store = rustls::RootCertStore::empty();
                for cert in certs {
                    cert_store.add(cert)?;
                }

                let config = ::quinn::ClientConfig::with_root_certificates(cert_store);

                let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
                endpoint.set_default_client_config(config);

                endpoint
            },
        })
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Opcode = Opcode;

    fn account_me(&self) -> AccountRef {
        self.account_me.account_ref()
    }

    fn account_primary(&self) -> Result<AccountRef> {
        self.account_primary
            .ok_or_else(|| anyhow!("failed to get primary address"))
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
        // connect to the target
        let conn = self.get_connection(target).await?;
        let (mut send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| anyhow!("failed to open stream: {}", e))?;

        // send opcode
        send.write_u8(opcode.bits()).await?;

        // send data
        ipis::tokio::io::copy(msg, &mut send)
            .await
            .map_err(|e| anyhow!("failed to send request: {}", e))?;

        // finish sending
        send.finish()
            .await
            .map_err(|e| anyhow!("failed to shutdown stream: {}", e))?;

        // be ready for receiving
        Ok(Box::pin(recv))
    }
}

impl IpiisClient {
    pub fn add_address(&self, target: AccountRef, address: SocketAddr) -> Result<()> {
        self.address_db
            .insert(target.as_bytes(), address.to_string().into_bytes())
            .map(|_| ())
            .map_err(Into::into)
    }

    pub(crate) async fn get_address(&self, target: &AccountRef) -> Result<SocketAddr> {
        match self.address_db.get(target.as_bytes())? {
            Some(addr) => Ok(String::from_utf8(addr.to_vec())?.parse()?),
            None => match self.account_primary() {
                Ok(primary) => self
                    .call_deserialized(Opcode::ARP, &primary, &ArpRequest { target: *target })
                    .await
                    .map(|res: ArpResponse| res.addr),
                Err(e) => bail!("{}: failed to get address: {}", e, target.to_string()),
            },
        }
    }

    async fn get_connection(&self, target: &AccountRef) -> Result<Connection> {
        let addr = self.get_address(target).await?;
        let server_name = cert::get_name(target);

        let new_conn = self
            .endpoint
            .connect(addr, &server_name)?
            .await
            .map_err(|e| anyhow!("failed to connect: {}", e))?;

        let quinn::NewConnection {
            connection: conn, ..
        } = new_conn;

        Ok(conn)
    }
}
