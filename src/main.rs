mod cli;
mod dns;
mod stats;
mod protocol;

use clap::Parser;
use protocol::ping::{ping, seek_send_bytes};

use std::io::ErrorKind;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cli::{validate_args, Args};
use dns::{resolve_host_with_family, strip_port, IpFamily};
use stats::Total;

fn main() {
    let args = Args::parse();
    if let Err(message) = validate_args(&args) {
        eprintln!("{message}");
        return;
    }

    let family = if args.ipv4 {
        IpFamily::V4
    } else if args.ipv6 {
        IpFamily::V6
    } else {
        IpFamily::Any
    };

    let addr = match resolve_host_with_family(&args.target, family) {
        Ok(addr) => addr,
        Err(err) => {
            eprintln!("{err}");
            return;
        }
    };
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
        match ping(host, &addr, timeout, args.verify) {
            Ok(r) => {
                total.received += 1;
                total.rtts.push(r.elapsed);

                if args.json {
                    println!("{}", r.json);
                    return;
                }

                let mut info = format!(
                    "{} bytes from {}: seq={} ",
                    r.received_bytes,
                    addr.ip(),
                    seq + 1
                );

                if let Some(onlines) = r.onlines {
                    info.push_str(&format!("onlines={onlines} "));
                }

                if let Some(mods) = r.mods {
                    info.push_str(&format!("mods={mods} "));
                }

                info.push_str(&format!("time={}ms", r.elapsed.as_millis()));
                println!("{info}");
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
        println!("\n--- {host} statistics ---");
        println!("{total}");
    }
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
