pub extern crate ipnet;

use std::{process::Command, time::Duration};

use ipis::core::anyhow::Result;
use ipnet::IpNet;

#[derive(Default)]
pub struct Simulator {
    network_delay: bool,
}

impl Simulator {
    pub fn apply_network_delay(&mut self, delay: Duration, destination: IpNet) -> Result<()> {
        // enable flag
        self.network_delay = true;

        // external call
        let output = Command::new("sh")
            .arg("-c")
            .arg(format!(
                r#"
for interface in $(
    ip address |
        grep 'state UP' |
        egrep -o '^[0-9]+\: (en[0-9a-z]+)' |
        sed 's/.* \(en.*\)/\1/g' |
        cat
); do
    tc qdisc del dev $interface root # Ensure you start from a clean state
    tc qdisc add dev $interface root handle 1: prio
    tc qdisc add dev $interface parent 1:1 handle 30: netem delay {delay}ms
    tc filter add dev $interface protocol ip parent 1:0 prio 1 u32 match ip dst {dst} flowid 1:1
done
"#,
                delay = delay.as_millis(),
                dst = destination.to_string(),
            ))
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            panic!(
                "Failed to apply the network delay: {}",
                String::from_utf8_lossy(&output.stderr),
            )
        }
    }

    pub fn clear_network_delay(&mut self) -> Result<()> {
        // disable flag
        if !self.network_delay {
            return Ok(());
        }
        self.network_delay = false;

        // external call
        let output = Command::new("sh")
            .arg("-c")
            .arg(
                r#"
for interface in $(
    ip address |
        grep 'state UP' |
        egrep -o '^[0-9]+\: (en[0-9a-z]+)' |
        sed 's/.* \(en.*\)/\1/g' |
        cat
); do
    tc qdisc del dev $interface root # Ensure you start from a clean state
done
"#,
            )
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            panic!(
                "Failed to clear the network delay: {}",
                String::from_utf8_lossy(&output.stderr),
            )
        }
    }
}

impl Drop for Simulator {
    fn drop(&mut self) {
        self.clear_network_delay().unwrap();
    }
}
