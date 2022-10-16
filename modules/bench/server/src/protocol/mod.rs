mod quic;
mod tcp;

use std::sync::Arc;

use ipiis_common::Ipiis;
use ipiis_modules_bench_common::args;
use ipis::{
    core::{account::GuaranteeSigned, anyhow::Result, data::Data},
    env::Infer,
    stream::DynStream,
    tokio::io::AsyncRead,
};

pub struct ProtocolImpl<IpiisServer> {
    client: Arc<IpiisServer>,
}

impl<IpiisServer> ::core::ops::Deref for ProtocolImpl<IpiisServer> {
    type Target = IpiisServer;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl<IpiisServer> ProtocolImpl<IpiisServer>
where
    IpiisServer: Ipiis,
{
    async fn handle_ping<R>(
        client: &IpiisServer,
        mut recv: R,
    ) -> Result<::ipiis_modules_bench_common::io::response::Ping<'static>>
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        // recv sign
        let sign_as_guarantee: Data<GuaranteeSigned, u8> =
            DynStream::recv(&mut recv).await?.into_owned().await?;

        // recv data
        let _ = DynStream::<Vec<u8>>::recv(recv).await?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_modules_bench_common::io::response::Ping {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
        })
    }
}

pub async fn select(args: &args::ArgsServer) {
    match args.inputs.protocol {
        args::ArgsProtocol::Quic => {
            ProtocolImpl {
                client: Arc::new(::ipiis_api_quic::server::IpiisServer::infer().await),
            }
            .run()
            .await
        }
        args::ArgsProtocol::Tcp => {
            ProtocolImpl {
                client: Arc::new(::ipiis_api_tcp::server::IpiisServer::infer().await),
            }
            .run()
            .await
        }
    }
}
