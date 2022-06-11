use core::{
    pin::Pin,
    task::{Context, Poll},
};

use ipis::tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};

#[repr(C)]
pub struct IpiisReader {
    cid: u64,
    len: u32,
}

impl IpiisReader {
    pub fn new(cid: u64, len: u32) -> Self {
        Self { cid, len }
    }
}

impl AsyncRead for IpiisReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let a = unsafe {
            let mut buf: u32 = 0;
            let mut len: u32 = 0;
            let _result = super::intrinsics::ipiis_reader__next(
                self.cid,
                (&mut buf) as *mut u32 as u32,
                (&mut len) as *mut u32 as u32,
            );

            ::core::slice::from_raw_parts(buf as *const u8, len as usize)
        };
        buf.put_slice(a);
        Poll::Ready(Ok(()))
    }
}

#[repr(C)]
pub struct IpiisWriter {
    cid: u64,
}

impl IpiisWriter {
    pub fn new(cid: u64) -> Self {
        Self { cid }
    }
}

impl AsyncWrite for IpiisWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, io::Error>> {
        let len = buf.len();

        unsafe {
            let _result = super::intrinsics::ipiis_writer__next(
                self.cid,
                buf.as_ptr() as *const u32 as u32,
                len as u32 as *const u32 as u32,
            );
        }
        Poll::Ready(Ok(len))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        unsafe {
            let _result = super::intrinsics::ipiis_writer__flush(self.cid);
        }
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        unsafe {
            let _result = super::intrinsics::ipiis_writer__shutdown(self.cid);
        }
        Poll::Ready(Ok(()))
    }
}
