use std::time::{Duration, Instant};

use tokio::io;

pub use config::{Config, config};
pub use mods::*;
pub use server::Server;

use crate::status::StatusWriter;

mod server;
mod config;
mod mods;
mod webhook;
mod status;

pub const CACHE_ROOT: &str = "mod_cache";

const MIN_INTERVAL: Duration = Duration::from_secs(4 * 60);

#[tokio::main]
pub async fn main() {
    loop {
        let config = config().await;

        let status = match &config.webhook {
            Some(webhook) => StatusWriter::from(webhook::Client::open(webhook)),
            None => StatusWriter::none(),
        };

        let mut mods = Mods::parse(&config);
        println!("parsed {} mods", mods.mods.len());

        println!("collecting mod jars...");

        let mod_jars = mods.collect_jars(&status).await;

        status.write("Starting up server...");

        println!("opening server...");

        let start = Instant::now();

        let mut server = Server::open(&config.server_jar);
        let jvm = config.jvm.as_ref().map(|jvm| jvm.as_str());

        let result = server.run(jvm, &mod_jars).await;
        eprintln!("server exited with result: {:?}", result);

        let interval = Instant::now() - start;
        if interval < MIN_INTERVAL {
            println!("server restarted very quickly! waiting a bit...");

            let delay = MIN_INTERVAL - interval;
            status.write(format!("Server restarted too quickly! Waiting for {} seconds...", delay.as_secs()));

            tokio::time::delay_for((delay).into()).await;
        } else {
            status.write("Server closed! Restarting...");
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Reqwest(reqwest::Error),
    TomlDe(toml::de::Error),
    Regex(fancy_regex::Error),
    MissingArtifact,
}

impl From<io::Error> for Error {
    fn from(io: io::Error) -> Self { Error::Io(io) }
}

impl From<reqwest::Error> for Error {
    fn from(reqwest: reqwest::Error) -> Self { Error::Reqwest(reqwest) }
}

impl From<toml::de::Error> for Error {
    fn from(toml: toml::de::Error) -> Self { Error::TomlDe(toml) }
}

impl From<fancy_regex::Error> for Error {
    fn from(regex: fancy_regex::Error) -> Self { Error::Regex(regex) }
}
