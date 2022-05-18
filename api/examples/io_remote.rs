use std::sync::Arc;

use ipiis_api::{common::Ipiis, server::IpiisServer};
use ipis::{
    core::{
        account::{Account, AccountRef},
        anyhow::Result,
        value::hash::Hash,
    },
    env::Infer,
    tokio,
};

async fn deploy(port: u16, parent: Option<(AccountRef, u16)>) -> Result<Arc<IpiisServer>> {
    // register the parent account
    if let Some((account, port)) = parent {
        ::std::env::set_var("ipiis_account_primary", account.to_string());
        ::std::env::set_var("ipiis_account_primary_address", format!("127.0.0.1:{port}"));
    }

    // create a server
    let server = Arc::new(IpiisServer::genesis(port).await?);

    // deploy the server
    tokio::spawn({
        let server = server.clone();
        async move { server.run_ipiis().await }
    });
    Ok(server)
}

#[tokio::main]
async fn main() -> Result<()> {
    // deploy a centralized server
    let center_1 = deploy(5001, None).await?;
    let center_1_account = center_1.account_me().account_ref();

    // deploy a edge
    let edge_1 = deploy(5002, Some((center_1_account, 5001))).await?;
    let edge_1_account = edge_1.account_me().account_ref();

    // deploy a end
    let end_1 = deploy(5003, Some((edge_1_account, 5002))).await?;

    // get the center's account from `end_1`
    // route: `end_1` --> `edge_1` --> `center_1`
    assert_eq!(
        end_1.get_address(&center_1_account).await?.to_string(),
        "127.0.0.1:5001",
    );

    // let's put a dummy primary account in the `center_1`.
    let kind = Hash::with_str("my kind");
    let kind_account = Account::generate();
    center_1
        .set_account_primary(Some(&kind), &kind_account.account_ref())
        .await?;

    // get the `kind`'s account from `end_1`
    // route: `end_1` --> `edge_1` --> `center_1`
    assert_eq!(
        end_1.get_account_primary(Some(&kind)).await?,
        kind_account.account_ref(),
    );

    Ok(())
}
