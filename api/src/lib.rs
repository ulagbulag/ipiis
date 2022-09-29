pub extern crate ipiis_common as common;

#[cfg(not(target_os = "wasi"))]
#[cfg(feature = "quic")]
pub use ipiis_api_quic::*;
#[cfg(not(target_os = "wasi"))]
#[cfg(feature = "tcp")]
pub use ipiis_api_tcp::*;

#[cfg(target_os = "wasi")]
pub mod client {
    pub use ipiis_api_wasi::IpiisClient;
}
