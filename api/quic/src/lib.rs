#[cfg(not(target_os = "wasi"))]
pub extern crate rustls;

pub mod common;
#[cfg(not(target_os = "wasi"))]
mod native;
#[cfg(target_os = "wasi")]
mod wasi;

use std::sync::Arc;

use ipiis_common::{handle_external_call, Ipiis, ServerResult};
use ipis::core::anyhow::Result;

#[cfg(not(target_os = "wasi"))]
pub use self::native::*;
#[cfg(target_os = "wasi")]
pub use self::wasi::*;

use crate::{client::IpiisClient, server::IpiisServer};

impl AsRef<Self> for IpiisClient {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<IpiisClient> for IpiisServer {
    fn as_ref(&self) -> &IpiisClient {
        self
    }
}

impl AsRef<Self> for IpiisServer {
    fn as_ref(&self) -> &Self {
        self
    }
}

handle_external_call!(
    server: IpiisServer => IpiisServer,
    request: ::ipiis_common::io => {
        GetAccountPrimary => handle_get_account_primary,
        SetAccountPrimary => handle_set_account_primary,
        GetAddress => handle_get_address,
        SetAddress => handle_set_address,
    },
);

impl IpiisServer {
    pub async fn run_ipiis(self: Arc<Self>) {
        let client = self.clone();

        self.run(client, Self::__handle::<IpiisClient>).await
    }

    async fn handle_get_account_primary(
        client: &IpiisServer,
        req: ::ipiis_common::io::request::GetAccountPrimary<
            'static,
            <IpiisClient as Ipiis>::Address,
        >,
    ) -> Result<
        ::ipiis_common::io::response::GetAccountPrimary<'static, <IpiisClient as Ipiis>::Address>,
    > {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let kind = sign_as_guarantee.data.data;

        // handle data
        let account = client.get_account_primary(kind.as_ref()).await?;
        let address = client.book.get(kind.as_ref(), &account)?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_common::io::response::GetAccountPrimary {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            account: ::ipis::stream::DynStream::Owned(account),
            address: ::ipis::stream::DynStream::Owned(address),
        })
    }

    async fn handle_set_account_primary(
        client: &IpiisServer,
        req: ::ipiis_common::io::request::SetAccountPrimary<'static>,
    ) -> Result<::ipiis_common::io::response::SetAccountPrimary<'static>> {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // verify as root
        sign_as_guarantee.ensure_self_signed()?;

        // unpack data
        let kind = sign_as_guarantee.data.data.0;
        let account = sign_as_guarantee.data.data.1;

        // handle data
        client.set_account_primary(kind.as_ref(), &account).await?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_common::io::response::SetAccountPrimary {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
        })
    }

    async fn handle_get_address(
        client: &IpiisServer,
        req: ::ipiis_common::io::request::GetAddress<'static, <IpiisClient as Ipiis>::Address>,
    ) -> Result<::ipiis_common::io::response::GetAddress<'static, <IpiisClient as Ipiis>::Address>>
    {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let kind = sign_as_guarantee.data.data.0;
        let account = sign_as_guarantee.data.data.1;

        // handle data
        let address = client.get_address(kind.as_ref(), &account).await?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_common::io::response::GetAddress {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            address: ::ipis::stream::DynStream::Owned(address),
        })
    }

    async fn handle_set_address(
        client: &IpiisServer,
        req: ::ipiis_common::io::request::SetAddress<'static, <IpiisClient as Ipiis>::Address>,
    ) -> Result<::ipiis_common::io::response::SetAddress<'static, <IpiisClient as Ipiis>::Address>>
    {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // verify as root
        sign_as_guarantee.ensure_self_signed()?;

        // unpack data
        let kind = sign_as_guarantee.data.data.0;
        let account = sign_as_guarantee.data.data.1;
        let address = sign_as_guarantee.data.data.2;

        // handle data
        client
            .set_address(kind.as_ref(), &account, &address)
            .await?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_common::io::response::SetAddress {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
        })
    }
}
