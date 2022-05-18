use ipiis_api::server::IpiisServer;
use ipis::{env::Infer, tokio};

#[tokio::main]
async fn main() {
    IpiisServer::infer().await.run_ipiis().await
}
