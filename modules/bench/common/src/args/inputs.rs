use std::path::PathBuf;

use byte_unit::Byte;
use clap::{Parser, ValueEnum};
use ipis::core::account::AccountRef;
use serde::{Deserialize, Serialize};
use simulation::ipnet::IpNet;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ArgsClient {
    #[clap(flatten)]
    pub ipiis: ArgsIpiis,
    #[clap(flatten)]
    pub inputs: ArgsClientInputs,
    #[clap(flatten)]
    pub simulation: ArgsSimulation,
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct ArgsServer {
    #[clap(flatten)]
    pub ipiis: ArgsIpiis,
    #[clap(flatten)]
    pub inputs: ArgsServerInputs,
}

#[derive(Debug, Parser)]
pub struct ArgsIpiis {
    /// Account of the target server
    #[clap(long, env = "ipiis_client_account_primary")]
    pub account: AccountRef,

    /// Address of the target server
    #[clap(long, env = "ipiis_client_account_primary_address")]
    pub address: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Parser)]
pub struct ArgsClientInputs {
    /// Protocol of benchmarking stream
    #[clap(value_enum)]
    #[clap(short, long, env = "PROTOCOL", default_value_t = ArgsProtocol::Tcp)]
    pub protocol: ArgsProtocol,

    /// Size of benchmarking stream
    #[clap(short, long, env = "DATA_SIZE", default_value_t = Byte::from_bytes(64_000_000))]
    pub size: Byte,

    /// Number of iteration
    #[clap(short, long, env = "NUM_ITERATIONS", default_value_t = Byte::from_bytes(30))]
    pub iter: Byte,

    /// Number of threads
    #[clap(long, env = "NUM_THREADS", default_value_t = 1)]
    pub num_threads: u32,

    /// Directory to save the results (filename is hashed by protocol and starting time)
    #[clap(long, env = "SAVE_DIR")]
    pub save_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Parser)]
pub struct ArgsServerInputs {
    /// Protocol of benchmarking stream
    #[clap(value_enum)]
    #[clap(short, long, env = "PROTOCOL", default_value_t = ArgsProtocol::Tcp)]
    pub protocol: ArgsProtocol,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ArgsProtocol {
    Quic,
    Tcp,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Parser)]
pub struct ArgsSimulation {
    /// Manual network delay in milliseconds
    #[clap(long, env = "SIMULATION_NETWORK_DELAY_MS")]
    pub network_delay_ms: Option<u64>,

    /// Manual network delay subnet
    #[clap(long, env = "SIMULATION_NETWORK_DELAY_SUBNET")]
    pub network_delay_subnet: Option<IpNet>,
}
