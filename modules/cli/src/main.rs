mod args;

use clap::Parser;
use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipis::{core::anyhow::Result, env::Infer, tokio};

#[tokio::main]
async fn main() -> Result<()> {
    // init logger
    ::ipis::logger::init_once();

    // parse the command-line arguments
    let args = args::Args::parse();

    // init client
    let client = IpiisClient::try_infer().await?;

    // execute a command
    match args.command {
        args::Command::SetAccount {
            kind,
            account,
            address,
            is_primary,
        } => {
            client
                .set_address(kind.as_ref(), &account, &address)
                .await?;
            if is_primary {
                client.set_account_primary(kind.as_ref(), &account).await?;
            }
            Ok(())
        }
    }
}
