mod protocol;

use ipiis_modules_bench_common::{args, clap::Parser};
use ipis::tokio;

#[tokio::main]
async fn main() {
    // init logger
    ::ipis::logger::init_once();

    // parse the command-line arguments
    let args = args::ArgsServer::parse();

    // deploy the server
    self::protocol::select(&args).await
}
