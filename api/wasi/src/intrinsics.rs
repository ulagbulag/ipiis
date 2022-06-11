extern "C" {
    pub fn ipiis_client_new() -> u64;

    pub fn ipiis_client_account_me() -> u32;

    pub fn ipiis_reader__next(cid: u64, buf: u32, len: u32) -> bool;
    pub fn ipiis_writer__next(cid: u64, buf: u32, len: u32) -> bool;
    pub fn ipiis_writer__flush(cid: u64) -> bool;
    pub fn ipiis_writer__shutdown(cid: u64) -> bool;
}
