use clap::{Parser, Subcommand};
use ipiis_api::{client::IpiisClient, common::Ipiis};
use ipis::core::account::AccountRef;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Subcommand)]
pub enum Command {
    GetAccount {
        /// Kind of the target server
        #[clap(long, env = "ipiis_client_kind")]
        kind: Option<String>,

        /// Account of the target server
        #[clap(long, env = "ipiis_client_account")]
        account: Option<AccountRef>,
    },
    SetAccount {
        /// Kind of the target server
        #[clap(long, env = "ipiis_client_kind")]
        kind: Option<String>,

        /// Account of the target server
        #[clap(long, env = "ipiis_client_account")]
        account: AccountRef,

        /// Address of the target server
        #[clap(long, env = "ipiis_client_address")]
        address: <IpiisClient as Ipiis>::Address,

        /// Whether the target server is primary
        #[clap(long, env = "ipiis_client_is_primary")]
        primary: bool,
    },
    DeleteAccount {
        /// Kind of the target server
        #[clap(long, env = "ipiis_client_kind")]
        kind: Option<String>,

        /// Account of the target server
        #[clap(long, env = "ipiis_client_account")]
        account: Option<AccountRef>,
    },
}
