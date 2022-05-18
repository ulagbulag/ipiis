use std::sync::Arc;

use ipiis_api::server::IpiisServer;
use ipis::{env::Infer, tokio};

#[tokio::main]
async fn main() {
    Arc::new(IpiisServer::infer().await).run_ipiis().await
}
