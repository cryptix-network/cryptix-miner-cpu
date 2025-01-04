use crate::Error;
use clap::{ArgGroup, Parser};
use log::LevelFilter;
use std::{net::IpAddr, str::FromStr};

#[derive(Debug, Parser)]
#[clap(about, version, author)]
#[clap(group(ArgGroup::new("required")))]
pub struct Opt {
    #[clap(short, long, display_order = 3)]
    pub debug: bool,
    
    #[clap(short = 'a', long = "mining-address", display_order = 0)]
    pub mining_address: String,
    
    #[clap(short = 's', long = "cryptixd-address", default_value = "127.0.0.1", display_order = 1)]
    pub cryptixd_address: String,

    #[clap(long = "devfund", display_order = 6, hide = true)]  
    pub devfund_address: Option<String>,

    #[clap(long = "devfund-percent", default_value = "1", display_order = 7, value_parser = parse_devfund_percent)]
    pub devfund_percent: u16,
    
    #[clap(short, long, display_order = 2)]
    pub port: Option<u16>,

    #[clap(long, display_order = 4)]
    pub testnet: bool,

    #[clap(short = 't', long = "threads", display_order = 5)]
    pub num_threads: Option<u16>,

    #[clap(long = "mine-when-not-synced", display_order = 8)]
    pub mine_when_not_synced: bool,

    #[clap(long = "throttle", display_order = 9)]
    pub throttle: Option<u64>,

    #[clap(long, display_order = 10)]
    pub altlogs: bool,
}

fn parse_devfund_percent(s: &str) -> Result<u16, &'static str> {
    let err = "devfund-percent should be --devfund-percent=XX.YY up to 2 numbers after the dot";
    let mut splited = s.split('.');
    let prefix = splited.next().ok_or(err)?;
    let postfix = splited.next().ok_or(err).unwrap_or("0");
    if splited.next().is_some() {
        return Err(err);
    };
    if prefix.len() > 2 || postfix.len() > 2 {
        return Err(err);
    }

    let postfix: u16 = postfix.parse().map_err(|_| err)?;
    let prefix: u16 = prefix.parse().map_err(|_| err)?;

    if prefix >= 100 || postfix >= 100 || (prefix == 0 && postfix == 0) {
        return Err("devfund-percent must be at least 1%");
    }

    Ok(prefix * 100 + postfix)
}

impl Opt {
    pub fn process(&mut self) -> Result<(), Error> {
        if self.cryptixd_address.is_empty() {
            self.cryptixd_address = "127.0.0.1".to_string();
        }

        if !self.cryptixd_address.starts_with("grpc://") {
            IpAddr::from_str(&self.cryptixd_address)?;
            let port = self.port();
            self.cryptixd_address = format!("grpc://{}:{}", self.cryptixd_address, port);
        }
        log::info!("Cryptixd address: {}", self.cryptixd_address);

        Ok(())
    }

    fn port(&mut self) -> u16 {
        *self.port.get_or_insert(if self.testnet { 16210 } else { 16110 })
    }

    pub fn log_level(&self) -> LevelFilter {
        if self.debug {
            LevelFilter::Debug
        } else {
            LevelFilter::Info
        }
    }

    pub fn devfund_address(&self) -> &str {
        "cryptix:qrjefk2r8wp607rmyvxmgjansqcwugjazpu2kk2r7057gltxetdvk8gl9fs0w"
    }
}
