#[cfg(not(target_os = "wasi"))]
pub use ipiis_api_quic_native::*;

#[cfg(target_os = "wasi")]
pub use ipiis_api_quic_wasi::*;
