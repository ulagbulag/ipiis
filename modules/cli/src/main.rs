mod args;

use clap::Parser;
use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipis::{
    core::{anyhow::Result, value::hash::Hash},
    env::Infer,
    tokio,
};

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
        args::Command::GetAccountPrimary { kind } => {
            let kind = kind.as_ref().map(|kind| Hash::with_str(kind));

            let account = client.get_account_primary(kind.as_ref()).await?.to_string();
            println!("Account = {account}");
            Ok(())
        }
        args::Command::GetAddress { kind, account } => {
            let kind = kind.as_ref().map(|kind| Hash::with_str(kind));
            let target = match account {
                Some(account) => account,
                None => client.get_account_primary(kind.as_ref()).await?,
            };

            let address = client.get_address(kind.as_ref(), &target).await?;
            println!("Address = {address}");
            Ok(())
        }
        args::Command::SetAccount {
            kind,
            account,
            address,
            primary,
        } => {
            let kind = kind.as_ref().map(|kind| Hash::with_str(kind));

            client
                .set_address(kind.as_ref(), &account, &address)
                .await?;
            if primary {
                client.set_account_primary(kind.as_ref(), &account).await?;
            }
            Ok(())
        }
    }
}
