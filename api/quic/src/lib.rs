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
