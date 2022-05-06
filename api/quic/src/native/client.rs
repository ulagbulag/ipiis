use core::pin::Pin;
use std::net::SocketAddr;

use ipiis_common::Ipiis;
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef, GuaranteeSigned},
        anyhow::{anyhow, bail, Result},
    },
    env::{infer, Infer},
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

impl Infer for IpiisClient {
    type GenesisArgs = [Certificate];
    type GenesisResult = Self;

    fn infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_client_account_primary").ok();
        let certs = ::rustls_native_certs::load_native_certs()?
            .into_iter()
            .map(|e| Certificate(e.0))
            .collect::<Vec<_>>();

        Self::new(account_me, account_primary, certs.as_slice())
    }

    fn genesis(certs: &<Self as Infer>::GenesisArgs) -> Result<<Self as Infer>::GenesisResult> {
        // generate an account
        let account = Account::generate();

        // init a server
        Self::new(account, None, certs)
    }
}

impl IpiisClient {
    pub fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        certs: &[Certificate],
    ) -> Result<Self> {
        let endpoint = {
            let mut cert_store = ::rustls::RootCertStore::empty();
            for cert in certs {
                cert_store.add(cert)?;
            }

            let client_config = ::quinn::ClientConfig::with_root_certificates(cert_store);
            let addr = "0.0.0.0:0".parse()?;

            let mut endpoint = Endpoint::client(addr)?;
            endpoint.set_default_client_config(client_config);

            endpoint
        };

        Self::with_address_db_path(
            account_me,
            account_primary,
            "ipiis_client_address_db",
            endpoint,
        )
    }

    pub(crate) fn with_address_db_path<P>(
        account_me: Account,
        account_primary: Option<AccountRef>,
        path: P,
        endpoint: Endpoint,
    ) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        Ok(Self {
            account_me,
            account_primary,
            // TODO: allow to store in specific directory
            address_db: sled::open(tempfile::tempdir()?.path().join(path))?,
            endpoint,
        })
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Opcode = Opcode;

    fn account_me(&self) -> &Account {
        &self.account_me
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
                    .call_permanent_deserialized(
                        Opcode::ARP,
                        &primary,
                        ArpRequest { target: *target },
                    )
                    .await
                    .map(|res: GuaranteeSigned<ArpResponse>| res.addr),
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
