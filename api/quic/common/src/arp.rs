use std::net::SocketAddr;

use bytecheck::CheckBytes;
use ipis::core::account::AccountRef;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Copy, Clone, CheckBytes, Debug, PartialEq, Eq, Hash))]
pub struct ArpRequest {
    pub target: AccountRef,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Archive, Serialize, Deserialize)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Copy, Clone, CheckBytes, Debug, PartialEq, Eq, Hash))]
pub struct ArpResponse {
    pub addr: SocketAddr,
}
