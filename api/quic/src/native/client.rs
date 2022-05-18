use core::pin::Pin;
use std::sync::Arc;

use ipiis_common::{external_call, Ipiis, RequestType, Response};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef, GuaranteeSigned, Verifier},
        anyhow::{anyhow, bail, Result},
        value::hash::Hash,
    },
    env::{infer, Infer},
    pin::PinnedInner,
    tokio::io::{AsyncRead, AsyncReadExt},
};
use quinn::{Connection, Endpoint};

use crate::{book::AddressBook, common::cert};

#[derive(Clone)]
pub struct IpiisClient {
    pub(crate) book: AddressBook<<Self as Ipiis>::Address>,
    endpoint: Endpoint,
}

#[async_trait]
impl<'a> Infer<'a> for IpiisClient {
    type GenesisArgs = Option<AccountRef>;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_account_primary").ok();

        Self::new(account_me, account_primary).await
    }

    async fn genesis(
        account_primary: <Self as Infer>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        let account_primary = account_primary.or_else(|| infer("ipiis_account_primary").ok());

        // generate an account
        let account = Account::generate();

        // init a server
        Self::new(account, account_primary).await
    }
}

impl IpiisClient {
    pub async fn new(account_me: Account, account_primary: Option<AccountRef>) -> Result<Self> {
        let endpoint = {
            let crypto = ::rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_custom_certificate_verifier(super::cert::ServerVerification::new())
                .with_no_client_auth();
            let client_config = ::quinn::ClientConfig::new(Arc::new(crypto));

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
        .await
    }

    pub(crate) async fn with_address_db_path<P>(
        account_me: Account,
        account_primary: Option<AccountRef>,
        book_path: P,
        endpoint: Endpoint,
    ) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        let client = Self {
            book: AddressBook::new(account_me, book_path)?,
            endpoint,
        };

        // try to add the primary account's address
        if let Some(account_primary) = account_primary {
            client.book.set_primary(None, &account_primary)?;

            if let Ok(address) = infer("ipiis_account_primary_address") {
                client.book.set(&account_primary, &address)?;
            }
        }

        Ok(client)
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Address = ::std::net::SocketAddr;

    fn account_me(&self) -> &Account {
        &self.book.account_me
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        match self.book.get_primary(kind)? {
            Some(address) => Ok(address),
            None => match kind {
                Some(kind) => {
                    // next target
                    let primary = self.get_account_primary(None).await?;

                    // pack request
                    let req = RequestType::<<Self as Ipiis>::Address>::GetAccountPrimary {
                        kind: Some(*kind),
                    };

                    // external call
                    let (account, address) = external_call!(
                        call: self
                            .call_permanent_deserialized(&primary, req)
                            .await?,
                        response: Response<<Self as Ipiis>::Address> => GetAccountPrimary,
                        items: { account, address },
                    );

                    // store response
                    self.book.set_primary(Some(kind), &account)?;
                    if let Some(address) = address {
                        self.book.set(&account, &address)?;
                    }

                    // unpack response
                    Ok(account)
                }
                None => bail!("failed to get primary address"),
            },
        }
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        self.book.set_primary(kind, account)
    }

    async fn get_address(&self, target: &AccountRef) -> Result<<Self as Ipiis>::Address> {
        match self.book.get(target)? {
            Some(address) => Ok(address),
            None => match self.book.get_primary(None)? {
                Some(primary) => {
                    // pack request
                    let req =
                        RequestType::<<Self as Ipiis>::Address>::GetAddress { account: *target };

                    // external call
                    let (address,) = external_call!(
                        call: self
                            .call_permanent_deserialized(&primary, req)
                            .await?,
                        response: Response<<Self as Ipiis>::Address> => GetAddress,
                        items: { address },
                    );

                    // store response
                    self.book.set(target, &address)?;

                    // unpack response
                    Ok(address)
                }
                None => {
                    let addr = target.to_string();
                    bail!("failed to get address: {addr}")
                }
            },
        }
    }

    async fn set_address(
        &self,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        self.book.set(target, address)
    }

    async fn call_raw<Req>(
        &self,
        target: &AccountRef,
        msg: &mut Req,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>>
    where
        Req: AsyncRead + Send + Sync + Unpin,
    {
        // connect to the target
        let conn = self.get_connection(target).await?;
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| anyhow!("failed to open stream: {e}"))?;

        // send data
        ipis::tokio::io::copy(msg, &mut send)
            .await
            .map_err(|e| anyhow!("failed to send request: {e}"))?;

        // finish sending
        send.finish()
            .await
            .map_err(|e| anyhow!("failed to shutdown stream: {e}"))?;

        // receive the result flag
        match super::flag::Result::from_bits(recv.read_u8().await?) {
            // be ready for receiving the data
            Some(super::flag::Result::ACK_OK) => Ok(Box::pin(recv)),
            // parse the error
            Some(super::flag::Result::ACK_ERR) => {
                // create a buffer
                let mut buf = {
                    let len = recv.read_u64().await?;
                    vec![0; len.try_into()?]
                };

                // recv data
                recv.read_exact(&mut buf).await?;

                // unpack data
                let res = PinnedInner::<GuaranteeSigned<String>>::new(buf)?.deserialize_into()?;

                // verify data
                let () = res.verify(Some(self.account_me().account_ref()))?;

                bail!(res.data.data)
            }
            Some(flag) if flag.contains(super::flag::Result::ACK) => {
                bail!("unknown ACK flag: {flag:?}")
            }
            Some(_) | None => bail!("cannot parse the result of response"),
        }
    }
}

impl IpiisClient {
    async fn get_connection(&self, target: &AccountRef) -> Result<Connection> {
        let addr = self.get_address(target).await?;
        let server_name = cert::get_name(target);

        let new_conn = self
            .endpoint
            .connect(addr, &server_name)?
            .await
            .map_err(|e| anyhow!("failed to connect: {e}"))?;

        let quinn::NewConnection {
            connection: conn, ..
        } = new_conn;

        Ok(conn)
    }
}
