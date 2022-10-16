use std::{ops::Range, sync::Arc, time::Duration};

use ipiis_common::Ipiis;
use ipiis_modules_bench_common::{args, IpiisBench};
use ipis::{async_trait::async_trait, core::anyhow::Result, stream::DynStream, tokio};

mod quic;
mod tcp;

#[async_trait]
pub trait Protocol {
    async fn to_string(&self) -> Result<String>;

    async fn ping(&self, ctx: self::BenchmarkCtx) -> Result<()>;
}

pub async fn select(args: &args::ArgsClient) -> Result<Box<dyn Protocol>> {
    match args.inputs.protocol {
        args::ArgsProtocol::Quic => self::quic::ProtocolImpl::try_new(&args.ipiis)
            .await
            .map(|protocol| Box::new(protocol) as Box<dyn Protocol>),
        args::ArgsProtocol::Tcp => self::tcp::ProtocolImpl::try_new(&args.ipiis)
            .await
            .map(|protocol| Box::new(protocol) as Box<dyn Protocol>),
    }
}

pub(super) async fn ping<T>(client: &T, ctx: self::BenchmarkCtx) -> Result<()>
where
    T: Ipiis + IpiisBench,
{
    for range in ctx
        .dataset
        .iter()
        .skip(ctx.offset as usize)
        .step_by(ctx.num_threads)
    {
        // compose simulation environment
        if let Some(delay) = ctx.simulation.delay_ms.map(Duration::from_millis) {
            tokio::time::sleep(delay).await;
        }

        let data = unsafe {
            ::core::slice::from_raw_parts(ctx.data.as_ptr().add(range.start), ctx.size_bytes)
        };
        client.ping(DynStream::BorrowedSlice(data)).await?;
    }
    Ok(())
}

pub struct BenchmarkCtx {
    pub num_threads: usize,
    pub size_bytes: usize,
    pub simulation: args::ArgsSimulation,

    pub offset: u32,
    pub dataset: Arc<[Range<usize>]>,
    pub data: Arc<[u8]>,
}
