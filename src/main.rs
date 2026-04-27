mod cli;
mod dns;
mod stats;
mod protocol;

use clap::Parser;
use protocol::ping::{ping, seek_send_bytes};

use std::io::{self, ErrorKind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use cli::Args;
use dns::{resolve_host_with_family, strip_port};
use stats::Total;

fn main() {
    let args = Args::parse();

    // If a check error is encountered, just panic; I don't think formatting is necessary.
    args.validate_args().unwrap();
    let addr = resolve_host_with_family(&args.target, args.get_ip_family()).unwrap(); 
    let host = args.server_address.as_deref().unwrap_or_else(|| strip_port(&args.target));
    let port = args.server_port.unwrap_or_else(|| addr.port());
    let timeout = Duration::from_secs_f64(args.timeout);

    // In json mode, just output json.
    if args.json {
        match ping(&addr, host, port, timeout, args.verify) {
            Ok(r) => println!("{}", r.json),
            Err(e) => print_io_error(&e),
        }
        return;
    }


    // The amount of data that the client can send can be calculated in advance, without needing to obtain it after sending.
    println!("PING {} ({}) {} bytes of data.", args.target, addr, seek_send_bytes(host));
    let mut total = Total::default();
    let running = set_ctrlc();
    let start = Instant::now();
    for seq in 0..args.count {
        if !running.load(Ordering::SeqCst) {
            break;
        }

        total.transmitted += 1;
        match ping(&addr, host, port, timeout, args.verify) {
            Ok(r) => {
                total.received += 1;
                total.rtts.push(r.elapsed);
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
            Err(e) => print_io_error(&e),
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }
        if seq + 1 < args.count && args.interval > 0.0 {
            thread::sleep(Duration::from_secs_f64(args.interval));
        }
    }
    total.time = start.elapsed();
    if total.transmitted > 0 {
        println!("\n--- {host} statistics ---");
        println!("{total}");
    }
}

fn print_io_error(error: &io::Error) {
    if error.kind() == ErrorKind::WouldBlock || error.kind() == ErrorKind::TimedOut {
        eprintln!("Request timed out.");
    } else {
        eprintln!("{}", error)
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
