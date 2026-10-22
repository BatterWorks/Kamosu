//! The five environment variables, all optional, all read once before serving
//! (spec #33). A default install cannot be misconfigured: with nothing set,
//! Kamosu binds `0.0.0.0:5266`, stores everything under `/data`, logs at
//! `info`, and is nobody's Demo.
//!
//! Port 5266 is deliberate (2026-08-26): it spells KAMO on a phone keypad, is
//! unassigned by IANA, sits outside Linux's ephemeral range, and avoids the
//! recipe-app neighbourhood (8080 Tandoor, 9000 Mealie).

use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// Address to bind (`KAMOSU_ADDRESS`, default `0.0.0.0` so dev is reachable
    /// across the LAN).
    pub address: IpAddr,
    /// Port to listen on (`KAMOSU_PORT`, default 5266).
    pub port: u16,
    /// The one durable directory (`KAMOSU_DATA_DIR`, default `/data`). Inside the
    /// container the path is fixed and nobody using Kamosu ever types it; outside a
    /// container — the developer's machine — this override keeps `/data` from being created.
    pub data_dir: PathBuf,
    /// Log filter for `tracing_subscriber` (`KAMOSU_LOG`, default `info`).
    pub log_filter: String,
    /// Whether this instance is a public Demo (ADR 0052): anyone arrives
    /// signed in, and only cooking and shopping save. Named for what it costs
    /// (`KAMOSU_DEMO_ERASES_LIBRARY_DAILY`), since a Demo puts its library
    /// back every day (#404), and on only when the variable says `true`: a
    /// typo leaves a library alone rather than erasing it.
    pub demo: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Config {
            address: env_ip("KAMOSU_ADDRESS", "0.0.0.0"),
            port: env_num("KAMOSU_PORT", 5266),
            data_dir: PathBuf::from(env_str("KAMOSU_DATA_DIR", "/data")),
            log_filter: env_str("KAMOSU_LOG", "info"),
            demo: env_str(DEMO, "false").trim().eq_ignore_ascii_case("true"),
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

/// The variable that turns an instance into a Demo.
pub const DEMO: &str = "KAMOSU_DEMO_ERASES_LIBRARY_DAILY";

fn env_str(key: &str, default: &str) -> String {
    match std::env::var(key) {
        Ok(v) if !v.trim().is_empty() => v,
        _ => default.to_string(),
    }
}

fn env_num(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(default)
}

fn env_ip(key: &str, default: &str) -> IpAddr {
    std::env::var(key)
        .ok()
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| default.parse().expect("valid default address"))
}
