use core::time::Duration;
use std::{sync::Arc, time::Instant};

use ipiis_api::{
    client::IpiisClient,
    common::{define_io, external_call, handle_external_call, Ipiis, ServerResult, CLIENT_DUMMY},
    server::IpiisServer,
};
use ipis::{
    async_trait::async_trait,
    core::{
        account::{AccountRef, GuaranteeSigned, GuarantorSigned},
        anyhow::{Error, Result},
    },
    env::Infer,
    futures,
    stream::DynStream,
    tokio::{self, io::AsyncRead},
};

#[tokio::main]
async fn main() -> Result<()> {
    // init peers
    let server = run_server(5002).await?;
    let client = run_client(server, 5002).await?;

    let data_size: usize = 256_000_000;
    let num_threads = 1;
    let num_iter = 30;

    println!(
        "* Data Size: {}",
        ::byte_unit::Byte::from_bytes(data_size.try_into()?).get_appropriate_unit(false),
    );
    println!("* Number of Iteration: {num_iter}");

    // create a data
    let req = Arc::new(vec![42; data_size]);

    let mut workers = vec![];
    for _ in 0..num_threads {
        let client = client.clone();
        let req = req.clone();

        let worker = tokio::spawn(async move {
            let mut time_total = Duration::default();
            for _ in 0..num_iter {
                let req = DynStream::BorrowedSlice(&req);

                // external call
                let instant = Instant::now();
                let res = external_call!(
                    client: &client,
                    target: None => &server,
                    request: crate::io => Raw,
                    sign: client.sign(server, CLIENT_DUMMY)?,
                    inputs: {
                        data: req,
                    },
                    inputs_mode: none,
                    outputs: send,
                );
                let mut res: DynStream<()> = DynStream::recv(res).await?;
                res.to_owned().await?;
                time_total += instant.elapsed();
            }
            Result::<_, Error>::Ok(time_total)
        });
        workers.push(worker);
    }

    let time_total: Duration = futures::future::try_join_all(workers)
        .await?
        .into_iter()
        .try_fold(Duration::default(), |a, b| Result::<_, Error>::Ok(a + b?))?;

    let time_average = time_total / (num_threads * num_iter);
    println!("* Duration: {:?}", time_average);
    println!("* Estimated Speed: {}bps", {
        let mut bytes = ::byte_unit::Byte::from_bytes(
            ((8 * data_size) as f64 / time_average.as_secs_f64()) as u128,
        )
        .get_appropriate_unit(false)
        .to_string();
        bytes.pop();
        bytes
    },);
    Ok(())
}

async fn run_client(server: AccountRef, port: u16) -> Result<Arc<IpiisClient>> {
    // init a client
    let client = IpiisClient::genesis(None).await?;
    client
        .set_address(None, &server, &format!("127.0.0.1:{}", port).parse()?)
        .await?;
    Ok(Arc::new(client))
}

async fn run_server(port: u16) -> Result<AccountRef> {
    // init a server
    let server = PingPongServer::genesis(port).await?;
    let public_key = server.as_ref().account_me().account_ref();

    // accept a single connection
    tokio::spawn(async move { server.run().await });
    tokio::time::sleep(Duration::from_secs(1)).await;

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
    request: crate::io => { },
    request_raw: crate::io => {
        Raw => handle_raw,
    },
);

impl PingPongServer {
    async fn handle_raw(
        client: &IpiisServer,
        mut recv: impl AsyncRead + Send + Unpin + 'static,
    ) -> Result<crate::io::response::Raw<'static>> {
        // recv request
        let req = crate::io::request::Raw::recv(client, &mut recv).await?;

        // unpack sign
        let sign_as_guarantee = req.__sign.into_owned().await?;

        // unpack data
        let data = req.data;

        // sign data
        let sign = client.sign_as_guarantor(sign_as_guarantee)?;

        // pack data
        Ok(crate::io::response::Raw {
            __lifetime: Default::default(),
            __sign: ::ipis::stream::DynStream::Owned(sign),
            data,
        })
    }
}

define_io! {
    Raw {
        inputs: {
            data: Vec<u8>,
        },
        input_sign: GuaranteeSigned<u8>,
        outputs: {
            data: Vec<u8>,
        },
        output_sign: GuarantorSigned<u8>,
        generics: { },
    },
}
