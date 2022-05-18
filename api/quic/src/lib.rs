#![feature(more_qualified_paths)]

#[cfg(not(target_os = "wasi"))]
pub extern crate rustls;

pub mod common;
#[cfg(not(target_os = "wasi"))]
mod native;
#[cfg(target_os = "wasi")]
mod wasi;

use std::sync::Arc;

use ipiis_common::{Ipiis, Request, RequestType, Response};
use ipis::{core::anyhow::Result, pin::Pinned};

#[cfg(not(target_os = "wasi"))]
pub use self::native::*;
#[cfg(target_os = "wasi")]
pub use self::wasi::*;

impl AsRef<Self> for crate::client::IpiisClient {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<crate::client::IpiisClient> for crate::server::IpiisServer {
    fn as_ref(&self) -> &crate::client::IpiisClient {
        self
    }
}

impl AsRef<Self> for crate::server::IpiisServer {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl crate::server::IpiisServer {
    pub async fn run_ipiis(self: &Arc<Self>) {
        let client = self.clone();

        self.run(client, Self::handle_ipiis).await
    }

    async fn handle_ipiis(
        client: Arc<crate::server::IpiisServer>,
        req: Pinned<Request<<crate::client::IpiisClient as Ipiis>::Address>>,
    ) -> Result<Response<<crate::client::IpiisClient as Ipiis>::Address>> {
        // TODO: handle without deserializing
        let req = req.deserialize_into()?;

        match req.data.data {
            RequestType::GetAccountPrimary { kind } => {
                let account = client.get_account_primary(kind.as_ref()).await?;
                let address = client.book.get(&account)?;

                Ok(Response::GetAccountPrimary { account, address })
            }
            RequestType::SetAccountPrimary { kind, account } => {
                req.ensure_self_signed()?;

                client
                    .set_account_primary(kind.as_ref(), &account)
                    .await
                    .map(|()| Response::SetAccountPrimary)
            }
            RequestType::GetAddress { account } => Ok(Response::GetAddress {
                address: client.get_address(&account).await?,
            }),
            RequestType::SetAddress { account, address } => {
                req.ensure_self_signed()?;

                client
                    .set_address(&account, &address)
                    .await
                    .map(|()| Response::SetAddress)
            }
        }
    }
}
