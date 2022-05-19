use std::sync::Arc;

use ipiis_api_quic::{client::IpiisClient, common::Ipiis, server::IpiisServer};
use ipiis_common::{define_io, external_call, handle_external_call, ServerResult};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{AccountRef, GuaranteeSigned, GuarantorSigned},
        anyhow::{bail, Result},
    },
    env::Infer,
    tokio::{self, io::AsyncRead},
};

#[tokio::main]
async fn main() -> Result<()> {
    // init peers
    let server = run_server(5001).await?;
    let client = run_client(server, 5001).await?;

    // create a data
    let name = "Alice".to_string();
    let age = 42;

    for _ in 0..5 {
        // handle Ok
        {
            // external call
            let (msg,) = external_call!(
                client: &client,
                target: None => &server,
                request: crate::io => Ok,
                sign: client.sign(server, ())?,
                inputs: {
                    name: "Alice".to_string(),
                    age: 42,
                },
                outputs: { msg, },
            );

            // verify data
            assert_eq!(msg, format!("hello, {} years old {}!", &name, age));
        }

        // handle Err
        {
            let f_err = || async {
                // external call
                let (msg,) = external_call!(
                    client: &client,
                    target: None => &server,
                    request: crate::io => Err,
                    sign: client.sign(server, ())?,
                    inputs: {
                        name: "Alice".to_string(),
                        age: 42,
                    },
                    outputs: { msg, },
                );

                Result::<_, ::ipis::core::anyhow::Error>::Ok(msg)
            };

            // external call
            let msg = f_err().await.expect_err("failed to catch the error");

            // verify data
            assert_eq!(
                msg.to_string(),
                format!("hello, {} years old {}!", &name, age),
            );
        }

        // handle Raw
        {
            // external call
            let (msg,) = external_call!(
                client: &client,
                target: None => &server,
                request: crate::io => Raw,
                sign: client.sign(server, ())?,
                inputs: {
                    name: "Alice".to_string(),
                    age: 42,
                },
                outputs: { msg, },
            );

            // verify data
            assert_eq!(msg, format!("hello, {} years old {}!", &name, age));
        }
    }
    Ok(())
}

async fn run_client(server: AccountRef, port: u16) -> Result<IpiisClient> {
    // init a client
    let client = IpiisClient::genesis(None).await?;
    client
        .set_address(None, &server, &format!("127.0.0.1:{}", port).parse()?)
        .await?;
    Ok(client)
}

async fn run_server(port: u16) -> Result<AccountRef> {
    // init a server
    let server = PingPongServer::genesis(port).await?;
    let public_key = server.as_ref().account_me().account_ref();

    // accept a single connection
    let server = Arc::new(server);
    tokio::spawn(async move { server.run().await });

    Ok(public_key)
}

pub struct PingPongServer {
    client: Arc<IpiisServer>,
}

impl AsRef<IpiisClient> for PingPongServer {
    fn as_ref(&self) -> &IpiisClient {
        &self.client
    }
}

#[async_trait]
impl<'a> Infer<'a> for PingPongServer {
    type GenesisArgs = <IpiisServer as Infer<'a>>::GenesisArgs;
    type GenesisResult = Self;

    async fn try_infer() -> Result<Self> {
        Ok(Self {
            client: IpiisServer::try_infer().await?.into(),
        })
    }

    async fn genesis(
        args: <Self as Infer<'a>>::GenesisArgs,
    ) -> Result<<Self as Infer<'a>>::GenesisResult> {
        Ok(Self {
            client: IpiisServer::genesis(args).await?.into(),
        })
    }
}

handle_external_call!(
    server: PingPongServer => IpiisServer,
    name: run,
    request: crate::io => {
        Ok => handle_ok,
        Err => handle_err,
    },
    request_raw: crate::io => {
        Raw => handle_raw,
    },
);

impl PingPongServer {
    async fn handle_ok(
        client: &IpiisServer,
        req: crate::io::request::Ok<'static>,
    ) -> Result<crate::io::response::Ok<'static>> {
        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let name = req.name.into_owned().await?;
        let age = req.age.into_owned().await?;

        // handle data
        let msg = format!("hello, {} years old {}!", &name, age);

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(crate::io::response::Ok {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            msg: ::ipis::stream::DynStream::Owned(msg),
        })
    }

    async fn handle_err(
        _client: &IpiisServer,
        req: crate::io::request::Err<'static>,
    ) -> Result<crate::io::response::Err<'static>> {
        // unpack data
        let name = req.name.into_owned().await?;
        let age = req.age.into_owned().await?;

        // handle data
        let msg = format!("hello, {} years old {}!", &name, age);

        // raise an error
        bail!(msg)
    }

    async fn handle_raw(
        client: &IpiisServer,
        mut recv: impl AsyncRead + Send + Unpin + 'static,
    ) -> Result<(crate::io::response::Raw<'static>, AccountRef)> {
        // recv request
        let req = crate::io::request::Raw::recv(client, &mut recv).await?;

        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // find the guarantee
        let guarantee = sign_as_guarantee.guarantee.account;

        // unpack data
        let name = req.name.into_owned().await?;
        let age = req.age.into_owned().await?;

        // handle data
        let msg = format!("hello, {} years old {}!", &name, age);

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        let res = crate::io::response::Raw {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            msg: ::ipis::stream::DynStream::Owned(msg),
        };
        Ok((res, guarantee))
    }
}

define_io! {
    Ok {
        inputs: {
            name: String,
            age: u32,
        },
        input_sign: GuaranteeSigned<()>,
        outputs: {
            msg: String,
        },
        output_sign: GuarantorSigned<()>,
        generics: { },
    },
    Err {
        inputs: {
            name: String,
            age: u32,
        },
        input_sign: GuaranteeSigned<()>,
        outputs: {
            msg: String,
        },
        output_sign: GuarantorSigned<()>,
        generics: { },
    },
    Raw {
        inputs: {
            name: String,
            age: u32,
        },
        input_sign: GuaranteeSigned<()>,
        outputs: {
            msg: String,
        },
        output_sign: GuarantorSigned<()>,
        generics: { },
    },
}
