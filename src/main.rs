mod protocol;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;
use clap::{Parser};
use protocol::ping::*;
use std::io::ErrorKind;
use std::net::{SocketAddr, ToSocketAddrs};
use std::process::exit;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

const MINECRAFT_DEFAULT_PORT: &'static str = ":25565";

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles=get_styles())]
struct Args {
    /// Hostname or IP address to ping
    target: String,

    /// Number of mc-ping requests to send
    #[arg(short = 'c', long, default_value_t = 4)]
    count: u32,

    /// Timeout for each request in seconds
    #[arg(short = 't', long, default_value_t = 8.0)]
    timeout: f64,

    /// Interval between requests in seconds
    #[arg(short = 'i', long, default_value_t = 0.0)]
    interval: f64,

    /// Output results in JSON format
    #[arg(short = 'j', long, default_value_t = false, conflicts_with = "count")]
    json: bool,
}

#[derive(Debug, Default)]
struct Total {
    transmitted: usize,
    received: usize,
    time: Duration,
    rtts: Vec<Duration>,
}

fn main() {
    let args = Args::parse();
    if let Some(s) = check_args(&args) {
        println!("{s}");
        return;
    }


    let addr = resolve_host(&args.target);
    let host = strip_port(&args.target);
    let timeout = Duration::from_secs_f64(args.timeout);
    let mut total = Total::default();

    if !args.json {
        println!(
            "PING {} ({}) {} bytes of data.",
            args.target,
            addr,
            seek_send_bytes(host)
        );
    }
    
    let running = set_ctrlc();
    let start = Instant::now();
    for seq in 0..args.count {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        total.transmitted += 1;
        match ping(host, &addr, timeout) {
            Ok(r) => {
                total.received += 1;
                total.rtts.push(r.elapsed);

                if args.json {
                    println!("{}", r.json);
                    return;
                } else {
                    let mut info = String::from_str(&format!(
                        "{} bytes from {}: seq={} ",
                        r.received_bytes,
                        addr.ip(),
                        seq + 1
                    ))
                    .unwrap();

                    if let Some(onlines) = r.onlines {
                        info.push_str(&format!("onlines={} ", onlines));
                    }

                    if let Some(mods) = r.mods {
                        info.push_str(&format!("mods={} ", mods));
                    }

                    info.push_str(&format!("time={}ms", r.elapsed.as_millis()));
                    println!("{info}");
                }
            }
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut {
                    eprintln!("Request timed out.");
                } else {
                    eprintln!("{}", e)
                }
            }
        }
        if !running.load(Ordering::SeqCst) {
            break;
        }
        if seq + 1 < args.count && args.interval > 0.0 {
            thread::sleep(Duration::from_secs_f64(args.interval));
        }
    }
    total.time = start.elapsed();
    if !args.json && total.transmitted > 0 {
        println!("\n--- {host} mc ping statistics ---");
        println!("{}", total.format());
    }
}

fn check_args(args: &Args) -> Option<&'static str> {
    if args.count == 0 {
        return Some("ping: bad number of packets to transmit.");
    }
    if args.interval < 0.0 {
        return Some("ping: bad timing interval");
    }
    if args.timeout <= 0.0 {
        return Some("ping: bad timeout.");
    }
    None
}

fn set_ctrlc() -> Arc<AtomicBool> {
    let arc = Arc::new(AtomicBool::new(true));
    let r = arc.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    arc
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
            exit(0);
        }
    };
}

fn strip_port(s: &str) -> &str {
    s.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(host, _)| host)
        .unwrap_or(s)
}

impl Total {
    fn format(&self) -> String {
        let mut s = String::new();

        let transmitted = self.transmitted;
        let received = self.received;
        let loss_pct = (transmitted - received) as f64 * 100.0 / transmitted as f64;

        s.push_str(&format!(
            "{} packets transmitted, {} received, {:.1}% packet loss, time {}ms\n",
            transmitted,
            received,
            loss_pct,
            self.time.as_millis()
        ));

        if self.rtts.len() > 0 {
            let min = self.rtts.iter().min().unwrap().as_secs_f64() * 1000.0;
            let max = self.rtts.iter().max().unwrap().as_secs_f64() * 1000.0;
            let sum_ms: f64 = self.rtts.iter().map(|d| d.as_secs_f64() * 1000.0).sum();
            let avg = sum_ms / self.rtts.len() as f64;

            let mdev = (self
                .rtts
                .iter()
                .map(|d| {
                    let diff = (d.as_secs_f64() * 1000.0) - avg;
                    diff * diff
                })
                .sum::<f64>()
                / self.rtts.len() as f64)
                .sqrt();

            s.push_str(&format!(
                "rtt min/avg/max/mdev = {:.3}/{:.3}/{:.3}/{:.3}ms\n",
                min, avg, max, mdev,
            ));
        }

        s
    }
}


pub fn get_styles() -> clap::builder::Styles {
    Styles::styled()
    .header(AnsiColor::BrightGreen.on_default())
    .usage(AnsiColor::BrightGreen.on_default())
    .literal(AnsiColor::BrightCyan.on_default())
    .placeholder(AnsiColor::Cyan.on_default())
}
