use std::{net::ToSocketAddrs, sync::Arc, time::Duration};

use ipiis_api_common::router::RouterClient;
use ipiis_common::{external_call, Ipiis};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, AccountRef},
        anyhow::{anyhow, bail, Result},
        value::hash::Hash,
    },
    env::{infer, Infer},
    resource::Resource,
};
use quinn::{Connection, Endpoint};

#[derive(Clone)]
pub struct IpiisClient {
    pub(crate) router: RouterClient<<Self as Ipiis>::Address>,
    endpoint: Endpoint,
}

#[async_trait]
impl<'a> Infer<'a> for IpiisClient {
    type GenesisArgs = Option<AccountRef>;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        let account_me = infer("ipis_account_me")?;
        let account_primary = infer("ipiis_account_primary").ok();

        Self::new(account_me, account_primary, None).await
    }

    async fn genesis(
        account_primary: <Self as Infer>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        let account_primary = account_primary.or_else(|| infer("ipiis_account_primary").ok());

        // generate an account
        let account = Account::generate();

        // init an endpoint
        Self::new(account, account_primary, None).await
    }
}

impl IpiisClient {
    pub async fn new(
        account_me: Account,
        account_primary: Option<AccountRef>,
        endpoint: Option<Endpoint>,
    ) -> Result<Self> {
        let endpoint = match endpoint {
            Some(endpoint) => endpoint,
            None => {
                let crypto = ::rustls::ClientConfig::builder()
                    .with_safe_defaults()
                    .with_custom_certificate_verifier(crate::cert::ServerVerification::new())
                    .with_no_client_auth();
                let client_config = {
                    let mut config = ::quinn::ClientConfig::new(Arc::new(crypto));
                    config.transport = {
                        let mut config = Arc::try_unwrap(config.transport).unwrap();
                        config.max_idle_timeout(Some(Duration::from_secs(10).try_into()?));
                        config.into()
                    };
                    config
                };

                let addr = "0.0.0.0:0".parse()?;

                let mut endpoint = Endpoint::client(addr)?;
                endpoint.set_default_client_config(client_config);

                endpoint
            }
        };

        let client = Self {
            router: RouterClient::new(account_me)?,
            endpoint,
        };

        // try to add the primary account's address
        if let Some(account_primary) = account_primary {
            client.router.set_primary(None, &account_primary)?;

            if let Ok(address) = infer("ipiis_account_primary_address") {
                client.router.set(None, &account_primary, &address)?;
            }
        }

        Ok(client)
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Address = String;
    type Reader = ::quinn::RecvStream;
    type Writer = ::quinn::SendStream;

    unsafe fn account_me(&self) -> Result<&Account> {
        Ok(&self.router.account_me)
    }

    fn account_ref(&self) -> &AccountRef {
        &self.router.account_ref
    }

    async fn get_account_primary(&self, kind: Option<&Hash>) -> Result<AccountRef> {
        match self.router.get_primary(kind)? {
            Some(address) => Ok(address),
            None => match kind {
                Some(kind) => {
                    // next target
                    let primary = self.get_account_primary(None).await?;

                    // external call
                    let (account, address) = external_call!(
                        client: self,
                        target: None => &primary,
                        request: ::ipiis_common::io => GetAccountPrimary,
                        sign: self.sign_owned(primary, Some(*kind))?,
                        inputs: { },
                        outputs: { account, address, },
                    );

                    // store response
                    self.router.set_primary(Some(kind), &account)?;
                    if let Some(address) = address {
                        self.router.set(Some(kind), &account, &address)?;
                    }

                    // unpack response
                    Ok(account)
                }
                None => bail!("failed to get primary address"),
            },
        }
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        self.router.set_primary(kind, account)?;

        // update server-side if you are a root
        if let Some(primary) = self.router.get_primary(None)? {
            if self.account_ref() == &primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => SetAccountPrimary,
                    sign: self.sign_owned(primary, (kind.copied(), *account))?,
                    inputs: { },
                );
            }
        }
        Ok(())
    }

    async fn delete_account_primary(&self, kind: Option<&Hash>) -> Result<()> {
        self.router.delete_primary(kind)?;

        // update server-side if you are a root
        if let Some(primary) = self.router.get_primary(None)? {
            if self.account_ref() == &primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => DeleteAccountPrimary,
                    sign: self.sign_owned(primary, kind.copied())?,
                    inputs: { },
                );
            }
        }
        Ok(())
    }

    async fn get_address(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<<Self as Ipiis>::Address> {
        match self.router.get(kind, target)? {
            Some(address) => Ok(address),
            None => match self.router.get_primary(None)? {
                Some(primary) => {
                    // external call
                    let (address,) = external_call!(
                        client: self,
                        target: None => &primary,
                        request: ::ipiis_common::io => GetAddress,
                        sign: self.sign_owned(primary, (kind.copied(), *target))?,
                        inputs: { },
                        outputs: { address, },
                    );

                    // store response
                    self.router.set(kind, target, &address)?;

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
        kind: Option<&Hash>,
        target: &AccountRef,
        address: &<Self as Ipiis>::Address,
    ) -> Result<()> {
        self.router.set(kind, target, address)?;

        // update server-side if you are a root
        if let Some(primary) = self.router.get_primary(None)? {
            if self.account_ref() == &primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => SetAddress,
                    sign: self.sign_owned(primary, (kind.copied(), *target, address.clone()))?,
                    inputs: { },
                );
            }
        }
        Ok(())
    }

    async fn delete_address(&self, kind: Option<&Hash>, target: &AccountRef) -> Result<()> {
        self.router.delete(kind, target)?;

        // update server-side if you are a root
        if let Some(primary) = self.router.get_primary(None)? {
            if self.account_ref() == &primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => DeleteAddress,
                    sign: self.sign_owned(primary, (kind.copied(), *target))?,
                    inputs: { },
                );
            }
        }
        Ok(())
    }

    fn protocol(&self) -> Result<String> {
        Ok("quic".to_string())
    }

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)> {
        // connect to the target
        let conn = self.get_connection(kind, target).await?;

        // open stream
        let (send, recv) = conn
            .open_bi()
            .await
            .map_err(|e| anyhow!("failed to open stream: {e}"))?;

        // send data
        Ok((send, recv))
    }
}

impl IpiisClient {
    async fn get_connection(&self, kind: Option<&Hash>, target: &AccountRef) -> Result<Connection> {
        let addr = self.get_address(kind, target).await?;
        let server_name = crate::cert::get_name(target);

        let new_conn = self
            .endpoint
            .connect(
                addr.to_socket_addrs()?
                    .next()
                    .ok_or_else(|| anyhow!("failed to parse the socket address: {addr}"))?,
                &server_name,
            )?
            .await
            .map_err(|e| anyhow!("failed to connect: {e}"))?;

        let quinn::NewConnection {
            connection: conn, ..
        } = new_conn;

        Ok(conn)
    }
}

#[async_trait]
impl Resource for IpiisClient {
    async fn release(&mut self) -> Result<()> {
        Ok(())
    }
}
