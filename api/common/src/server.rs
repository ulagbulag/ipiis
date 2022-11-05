#[macro_export]
macro_rules! impl_ipiis_server {
    (
        client: $client:ty,
        server: $server:ty,
    ) => {
        const _: () = {
            use std::sync::Arc;

            use ipiis_common::{handle_external_call, Ipiis, ServerResult};
            use ipis::core::anyhow::Result;

            impl AsRef<Self> for $client {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            impl AsRef<$client> for $server {
                fn as_ref(&self) -> &$client {
                    self
                }
            }

            impl AsRef<Self> for $server {
                fn as_ref(&self) -> &Self {
                    self
                }
            }

            handle_external_call!(
                server: $server => $server,
                request: ::ipiis_common::io => {
                    GetAccountPrimary => handle_get_account_primary,
                    SetAccountPrimary => handle_set_account_primary,
                    DeleteAccountPrimary => handle_delete_account_primary,
                    GetAddress => handle_get_address,
                    SetAddress => handle_set_address,
                    DeleteAddress => handle_delete_address,
                },
            );

            impl $server {
                pub async fn run_ipiis(self: Arc<Self>) {
                    let client = self.clone();

                    self.run(client, Self::__handle::<$client>).await
                }

                async fn handle_get_account_primary(
                    client: &$server,
                    req: ::ipiis_common::io::request::GetAccountPrimary<
                        'static,
                        <$client as Ipiis>::Address,
                    >,
                ) -> Result<
                    ::ipiis_common::io::response::GetAccountPrimary<
                        'static,
                        <$client as Ipiis>::Address,
                    >,
                > {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // unpack data
                    let kind = &sign_as_guarantee.data;

                    // handle data
                    let account = client.get_account_primary(kind.as_ref()).await?;
                    let address = client.router.get(kind.as_ref(), &account)?;

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
                    client: &$server,
                    req: ::ipiis_common::io::request::SetAccountPrimary<'static>,
                ) -> Result<::ipiis_common::io::response::SetAccountPrimary<'static>> {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // verify as root
                    sign_as_guarantee.metadata.ensure_self_signed()?;

                    // unpack data
                    let kind = sign_as_guarantee.data.0;
                    let account = sign_as_guarantee.data.1;

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

                async fn handle_delete_account_primary(
                    client: &$server,
                    req: ::ipiis_common::io::request::DeleteAccountPrimary<'static>,
                ) -> Result<::ipiis_common::io::response::DeleteAccountPrimary<'static>> {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // verify as root
                    sign_as_guarantee.metadata.ensure_self_signed()?;

                    // unpack data
                    let kind = sign_as_guarantee.data;

                    // handle data
                    client.delete_account_primary(kind.as_ref()).await?;

                    // sign data
                    let sign = client.sign_as_guarantor(sign_as_guarantee)?;

                    // pack data
                    Ok(::ipiis_common::io::response::DeleteAccountPrimary {
                        __lifetime: Default::default(),
                        __sign: ::ipis::stream::DynStream::Owned(sign),
                    })
                }

                async fn handle_get_address(
                    client: &$server,
                    req: ::ipiis_common::io::request::GetAddress<
                        'static,
                        <$client as Ipiis>::Address,
                    >,
                ) -> Result<
                    ::ipiis_common::io::response::GetAddress<'static, <$client as Ipiis>::Address>,
                > {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // unpack data
                    let kind = sign_as_guarantee.data.0;
                    let account = sign_as_guarantee.data.1;

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
                    client: &$server,
                    req: ::ipiis_common::io::request::SetAddress<
                        'static,
                        <$client as Ipiis>::Address,
                    >,
                ) -> Result<
                    ::ipiis_common::io::response::SetAddress<'static, <$client as Ipiis>::Address>,
                > {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // verify as root
                    sign_as_guarantee.metadata.ensure_self_signed()?;

                    // unpack data
                    let kind = sign_as_guarantee.data.0;
                    let account = sign_as_guarantee.data.1;
                    let address = &sign_as_guarantee.data.2;

                    // handle data
                    client.set_address(kind.as_ref(), &account, address).await?;

                    // sign data
                    let sign = client.sign_as_guarantor(sign_as_guarantee)?;

                    // pack data
                    Ok(::ipiis_common::io::response::SetAddress {
                        __lifetime: Default::default(),
                        __sign: ::ipis::stream::DynStream::Owned(sign),
                    })
                }

                async fn handle_delete_address(
                    client: &$server,
                    req: ::ipiis_common::io::request::DeleteAddress<'static>,
                ) -> Result<::ipiis_common::io::response::DeleteAddress<'static>> {
                    // unpack sign
                    let sign_as_guarantee = req.__sign.into_owned().await?;

                    // verify as root
                    sign_as_guarantee.metadata.ensure_self_signed()?;

                    // unpack data
                    let kind = sign_as_guarantee.data.0;
                    let account = sign_as_guarantee.data.1;

                    // handle data
                    client.delete_address(kind.as_ref(), &account).await?;

                    // sign data
                    let sign = client.sign_as_guarantor(sign_as_guarantee)?;

                    // pack data
                    Ok(::ipiis_common::io::response::DeleteAddress {
                        __lifetime: Default::default(),
                        __sign: ::ipis::stream::DynStream::Owned(sign),
                    })
                }
            }
        };
    };
}
