#[cfg(not(target_os = "wasi"))]
pub extern crate rustls;

pub mod common;
#[cfg(not(target_os = "wasi"))]
mod native;
#[cfg(target_os = "wasi")]
mod wasi;

#[cfg(not(target_os = "wasi"))]
pub use self::native::*;
#[cfg(target_os = "wasi")]
pub use self::wasi::*;

impl AsRef<Self> for crate::client::IpiisClient {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<crate::client::IpiisClient> for crate::server::IpiisServer {
    fn as_ref(&self) -> &crate::client::IpiisClient {
        self
    }
}

impl AsRef<Self> for crate::server::IpiisServer {
    fn as_ref(&self) -> &Self {
        self
    }
}
