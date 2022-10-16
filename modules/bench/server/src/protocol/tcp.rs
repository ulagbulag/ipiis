use std::sync::Arc;

use ipiis_api_tcp::{client::IpiisClient, server::IpiisServer};
use ipiis_common::{handle_external_call, Ipiis, ServerResult};
use ipis::core::anyhow::Result;

handle_external_call!(
    server: super::ProtocolImpl<IpiisServer> => IpiisServer,
    name: run,
    request: ::ipiis_modules_bench_common::io => { },
    request_raw: ::ipiis_modules_bench_common::io => {
        Ping => handle_ping,
    },
);
