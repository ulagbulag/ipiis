use std::{net::SocketAddr, sync::Arc};

use clap::{Parser, Subcommand};
use ipiis_api::{
    client::IpiisClient,
    common::{handle_external_call, Ipiis, ServerResult},
    server::IpiisServer,
};
use ipiis_modules_bench_common::{IpiisBench, KIND};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{Account, GuaranteeSigned},
        anyhow::Result,
    },
    env::Infer,
    log::info,
    stream::DynStream,
    tokio::{
        self,
        io::{AsyncRead, AsyncReadExt},
    },
};
use rand::{distributions::Uniform, Rng};

pub struct IpiisBenchServer {
    client: Arc<IpiisServer>,
}

impl ::core::ops::Deref for IpiisBenchServer {
    type Target = IpiisServer;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

#[async_trait]
impl<'a> Infer<'a> for IpiisBenchServer {
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
    server: IpiisBenchServer => IpiisServer,
    name: run,
    request: ::ipiis_modules_bench_common::io => { },
    request_raw: ::ipiis_modules_bench_common::io => {
        Ping => handle_ping,
    },
);

impl IpiisBenchServer {
    async fn handle_ping<R>(
        client: &IpiisServer,
        mut recv: R,
    ) -> Result<::ipiis_modules_bench_common::io::response::Ping<'static>>
    where
        R: AsyncRead + Send + Unpin + 'static,
    {
        // recv sign
        let sign_as_guarantee: GuaranteeSigned<()> =
            DynStream::recv(&mut recv).await?.into_owned().await?;

        // recv data
        let len = recv.read_u64().await?;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(::ipiis_modules_bench_common::io::response::Ping {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            data: ::ipis::stream::DynStream::Stream {
                len,
                recv: Box::pin(recv),
            },
        })
    }
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Commands,
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Subcommand)]
enum Commands {
    Client {
        /// Account of the target server
        #[clap(long)]
        account: Account,

        /// Address of the target server
        #[clap(long, default_value = "127.0.0.1:9999")]
        address: SocketAddr,

        /// Size of benchmarking stream
        #[clap(short, long, default_value_t = 1_000_000_000)]
        size: usize,
    },
    Server {
        /// Address of the server
        #[clap(short, long, default_value = "0.0.0.0")]
        address: String,

        /// Port of the server
        #[clap(short, long, default_value_t = 9999)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // init logger
    ::ipis::logger::init_once();

    // parse the command-line arguments
    let args = Args::parse();

    match args.command {
        Commands::Client {
            account,
            address,
            size,
        } => {
            // create a client
            let client = IpiisClient::genesis(None).await?;

            // registre the server account as primary
            client
                .set_account_primary(KIND.as_ref(), &account.account_ref())
                .await?;
            client
                .set_address(KIND.as_ref(), &account.account_ref(), &address)
                .await?;

            // init data
            info!("- Initializing...");
            let range = Uniform::from(0..=255);
            let data = ::rand::thread_rng()
                .sample_iter(&range)
                .take(size)
                .collect();
            let data = DynStream::OwnedVec(data);

            // begin benchmaring
            info!("- Benchmarking...");
            let duration = client.ping(data).await?;

            // print the output
            info!("- Finished!");
            info!("- Elapsed Time: {duration:?}");
            Ok(())
        }
        // deploy a server
        Commands::Server { address, port } => {
            // create a server
            let server = IpiisBenchServer::genesis(port).await?;

            // print the account
            info!("- Account: {}", server.account_me().to_string());
            info!("- Address: {address}:{port}");

            // deploy the server
            server.run().await;
            Ok(())
        }
    }
}
