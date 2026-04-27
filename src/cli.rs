use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::AnsiColor;

use crate::dns::IpFamily;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, styles=get_styles())]
pub struct Args {
    /// Hostname or IP address to ping
    pub target: String,

    /// Number of mc-ping requests to send
    #[arg(short = 'c', long, default_value_t = 4)]
    pub count: u32,

    /// Timeout for each request in seconds
    #[arg(short = 't', long, default_value_t = 8.0)]
    pub timeout: f64,

    /// Interval between requests in seconds
    #[arg(short = 'i', long, default_value_t = 0.0)]
    pub interval: f64,

    /// Verify ping-pong code
    #[arg(short = 'v', long, default_value_t = false)]
    pub verify: bool,

    /// Output results in JSON format
    #[arg(short = 'j', long, default_value_t = false, conflicts_with = "count")]
    pub json: bool,

    /// Force IPv4 DNS lookup
    #[arg(short = '4', long, conflicts_with = "ipv6")]
    pub ipv4: bool,

    /// Force IPv6 DNS lookup
    #[arg(short = '6', long, conflicts_with = "ipv4")]
    pub ipv6: bool,
}

impl Args {
    
    pub fn get_ip_family(&self) -> IpFamily {
        if self.ipv4 {
            return IpFamily::V4;
        } else if self.ipv6 {
            return IpFamily::V6;
        } else {
            return IpFamily::Any;
        }
    }
    
    pub fn validate_args(&self) -> Result<(), &'static str> {
        if self.count == 0 {
            return Err("ping: bad number of packets to transmit.");
        }
        if self.interval < 0.0 {
            return Err("ping: bad timing interval");
        }
        if self.timeout <= 0.0 {
            return Err("ping: bad timeout.");
        }
        Ok(())
    }   
}

fn get_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::BrightGreen.on_default())
        .usage(AnsiColor::BrightGreen.on_default())
        .literal(AnsiColor::BrightCyan.on_default())
        .placeholder(AnsiColor::Cyan.on_default())
}
