mod protocol;

use std::{sync::Arc, time::Instant};

use ipiis_modules_bench_common::{args, byte_unit::Byte, clap::Parser};
use ipis::{
    core::{anyhow::Result, chrono::Utc},
    futures,
    log::info,
    tokio,
};
use rand::{distributions::Uniform, Rng};

#[tokio::main]
async fn main() -> Result<()> {
    // init logger
    ::ipis::logger::init_once();

    // parse the command-line arguments
    let args = args::ArgsClient::parse();

    // log starting time
    let timestamp = Utc::now();
    info!("- Starting Time: {timestamp:?}");

    // init protocol
    let protocol = self::protocol::select(&args).await?;

    // print the configuration
    info!("- Account: {}", args.ipiis.account.to_string());
    info!("- Address: {}", &args.ipiis.address);
    info!("- Data Size: {}", args.inputs.size);
    info!("- Number of Iteration: {}", args.inputs.iter);
    info!("- Number of Threads: {}", args.inputs.num_threads);

    let size_bytes: usize = args.inputs.size.get_bytes().try_into()?;
    let num_iteration: usize = args.inputs.iter.get_bytes().try_into()?;
    let num_threads: usize = args.inputs.num_threads.try_into()?;

    let simulation = args.simulation;

    // init data
    info!("- Initializing...");
    let range = Uniform::from(0..=255);
    let data: Arc<[_]> = ::rand::thread_rng()
        .sample_iter(&range)
        .take(size_bytes + num_iteration)
        .collect::<Vec<u8>>()
        .into();

    // construct dataset
    info!("- Generating Dataset ...");
    let dataset: Arc<[_]> = (0..num_iteration)
        .map(|iter| (iter..iter + size_bytes))
        .collect();

    // begin benchmaring
    let duration = {
        info!("- Benchmarking ...");

        let instant = Instant::now();
        futures::future::try_join_all(
            (0..args.inputs.num_threads)
                .map(|offset| crate::protocol::BenchmarkCtx {
                    num_threads,
                    size_bytes,
                    simulation,

                    offset,
                    dataset: dataset.clone(),
                    data: data.clone(),
                })
                .map(|ctx| protocol.ping(ctx)),
        )
        .await?;
        instant.elapsed()
    };

    // collect results
    info!("- Collecting results ...");
    let outputs = args::ResultsOutputsMetric {
        elapsed_time_s: duration.as_secs_f64(),
        iops: num_iteration as f64 / duration.as_secs_f64(),
        speed_bps: (8 * size_bytes * num_iteration) as f64 / duration.as_secs_f64(),
    };

    // save results to a file
    if let Some(mut save_dir) = args.inputs.save_dir.clone() {
        let protocol = protocol.to_string().await?;
        let timestamp = timestamp.to_rfc3339();
        let filename = format!("benchmark-ipiis-{protocol}-{timestamp}.json");
        let filepath = {
            save_dir.push(filename);
            save_dir
        };

        info!("- Saving results to {filepath:?} ...");
        let results = args::Results {
            ipiis: args::ArgsIpiisPublic {
                account: args.ipiis.account.to_string(),
                address: args.ipiis.address,
            },
            inputs: args.inputs,
            outputs: outputs.clone(),
        };
        let file = ::std::fs::File::create(filepath)?;
        ::serde_json::to_writer(file, &results)?;
    }

    // print the output
    info!("- Finished!");
    info!("- Elapsed Time: {:?}", outputs.elapsed_time_s);
    info!("- IOPS: {}", outputs.iops);
    info!("- Speed: {}bps", {
        let mut speed = Byte::from_bytes(outputs.speed_bps as u128)
            .get_appropriate_unit(false)
            .to_string();
        speed.pop();
        speed
    });

    Ok(())
}
