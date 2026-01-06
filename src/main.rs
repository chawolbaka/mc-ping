mod protocol;
use clap::Parser;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process::exit;
use std::str::FromStr;
use protocol::ping::*;

const MINECRAFT_DEFAULT_PORT: &'static str = ":25565";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Hostname or IP address to ping
    target: String,

    /// Number of mc-ping requests to send
    #[arg(short = 'c', long, default_value_t = 4)]
    count: u32,
}

fn main() {
    let args = Args::parse();
    let addr = resolve_host(&args.target);
    for seq in 0..args.count {
        match ping(strip_port(&args.target), &addr) {
            Ok(r) => {
                let mut info = String::from_str(&format!(
                    "{} bytes from {}: seq={} ", r.received_bytes, addr.ip(), seq)).unwrap();

                if let Some(onlines) = r.online {
                    info.push_str(&format!("onlines={} ", onlines));
                }

                if let Some(mods) = r.mods {
                    info.push_str(&format!("mods={} ", mods));
                }

                info.push_str(&format!("time={:?}", r.elapsed));
                println!("{info}");
            },
            Err(e) => eprintln!("{}", e),
        }
    }
}



fn resolve_host(host: &str) -> SocketAddr {
    let mut host = String::from_str(host).unwrap();
    if !host.contains(':') {
        host.push_str(MINECRAFT_DEFAULT_PORT);
    }

    match host.to_socket_addrs() {
        Ok(mut addrs) => return addrs.next().unwrap(),
        Err(_) => {
            eprintln!(
                "Ping request could not find host {}. Please check the name and try again.",
                strip_port(&host)
            );
            exit(-1);
        }
    };
}

fn strip_port(s: &str) -> &str {
    s.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(s)
}
