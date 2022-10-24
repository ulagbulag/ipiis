use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use super::inputs::{ArgsClientInputs, ArgsSimulation};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Results {
    pub ipiis: ArgsIpiisPublic,
    pub inputs: ArgsClientInputs,
    pub outputs: ResultsOutputsMetric,
    pub simulation: ArgsSimulation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgsIpiisPublic {
    /// Public Account of the target server
    pub account: String,

    /// Address of the target server
    pub address: SocketAddr,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResultsOutputsMetric {
    /// Protocol of queried benchmarking stream
    pub protocol: String,

    /// Elapsed time as seconds
    pub elapsed_time_s: f64,

    /// I/O per seconds
    pub iops: f64,

    /// Estimated speed as bps
    pub speed_bps: f64,
}
