use ipiis_api_common::book::AddressBook;
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
    tokio,
};

#[derive(Clone)]
pub struct IpiisClient {
    pub(crate) book: AddressBook<<Self as Ipiis>::Address>,
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

        // init an endpoint
        Self::new(account, account_primary).await
    }
}

impl IpiisClient {
    pub async fn new(account_me: Account, account_primary: Option<AccountRef>) -> Result<Self> {
        Self::with_address_db_path(account_me, account_primary, "ipiis_client_address_db").await
    }

    pub(crate) async fn with_address_db_path<P>(
        account_me: Account,
        account_primary: Option<AccountRef>,
        book_path: P,
    ) -> Result<Self>
    where
        P: AsRef<::std::path::Path>,
    {
        let client = Self {
            book: AddressBook::new(account_me, book_path)?,
        };

        // try to add the primary account's address
        if let Some(account_primary) = account_primary {
            client.book.set_primary(None, &account_primary)?;

            if let Ok(address) = infer("ipiis_account_primary_address") {
                client.book.set(None, &account_primary, &address)?;
            }
        }

        Ok(client)
    }
}

#[async_trait]
impl Ipiis for IpiisClient {
    type Address = ::std::net::SocketAddr;
    type Reader = tokio::io::ReadHalf<tokio::net::TcpStream>;
    type Writer = tokio::io::WriteHalf<tokio::net::TcpStream>;

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

                    // external call
                    let (account, address) = external_call!(
                        client: self,
                        target: None => &primary,
                        request: ::ipiis_common::io => GetAccountPrimary,
                        sign: self.sign(primary, Some(*kind))?,
                        inputs: { },
                        outputs: { account, address, },
                    );

                    // store response
                    self.book.set_primary(Some(kind), &account)?;
                    if let Some(address) = address {
                        self.book.set(Some(kind), &account, &address)?;
                    }

                    // unpack response
                    Ok(account)
                }
                None => bail!("failed to get primary address"),
            },
        }
    }

    async fn set_account_primary(&self, kind: Option<&Hash>, account: &AccountRef) -> Result<()> {
        self.book.set_primary(kind, account)?;

        // update server-side if you are a root
        if let Some(primary) = self.book.get_primary(None)? {
            if self.account_me().account_ref() == primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => SetAccountPrimary,
                    sign: self.sign(primary, (kind.copied(), *account))?,
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
        match self.book.get(kind, target)? {
            Some(address) => Ok(address),
            None => match self.book.get_primary(None)? {
                Some(primary) => {
                    // external call
                    let (address,) = external_call!(
                        client: self,
                        target: None => &primary,
                        request: ::ipiis_common::io => GetAddress,
                        sign: self.sign(primary, (kind.copied(), *target))?,
                        inputs: { },
                        outputs: { address, },
                    );

                    // store response
                    self.book.set(kind, target, &address)?;

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
        self.book.set(kind, target, address)?;

        // update server-side if you are a root
        if let Some(primary) = self.book.get_primary(None)? {
            if self.account_me().account_ref() == primary {
                // external call
                external_call!(
                    client: self,
                    target: None => &primary,
                    request: ::ipiis_common::io => SetAddress,
                    sign: self.sign(primary, (kind.copied(), *target, *address))?,
                    inputs: { },
                );
            }
        }
        Ok(())
    }

    async fn call_raw(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<(<Self as Ipiis>::Writer, <Self as Ipiis>::Reader)> {
        // connect to the target
        let conn = self.get_connection(kind, target).await?;

        // open stream
        let (recv, send) = tokio::io::split(conn);

        // send data
        Ok((send, recv))
    }
}

impl IpiisClient {
    async fn get_connection(
        &self,
        kind: Option<&Hash>,
        target: &AccountRef,
    ) -> Result<tokio::net::TcpStream> {
        let addr = self.get_address(kind, target).await?;

        let new_conn = tokio::net::TcpSocket::new_v4()?
            .connect(addr)
            .await
            .map_err(|e| anyhow!("failed to connect: {e}"))?;

        Ok(new_conn)
    }
}

#[async_trait]
impl Resource for IpiisClient {
    async fn release(&mut self) -> Result<()> {
        Ok(())
    }
}
