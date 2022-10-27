use clap::{Parser, Subcommand};
use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipis::core::{account::AccountRef, value::hash::Hash};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    SetAccount {
        /// Kind of the target server
        #[clap(long, env = "ipiis_client_kind")]
        kind: Option<Hash>,

        /// Account of the target server
        #[clap(long, env = "ipiis_client_account")]
        account: AccountRef,

        /// Address of the target server
        #[clap(long, env = "ipiis_client_address")]
        address: <IpiisClient as Ipiis>::Address,

        /// Whether the target server is primary
        #[clap(long, env = "ipiis_client_is_primary")]
        is_primary: bool,
    },
}
