use clap::Parser;
use std::net::{IpAddr, SocketAddr};

#[derive(Parser)]
pub struct Args {
    #[clap(long, default_value_t = 5700)]
    pub port: u16,

    #[clap(long, default_value = "0.0.0.0")]
    pub host: IpAddr
}
