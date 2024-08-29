
use figment::{
    providers::{Env, Format, Toml},
    Figment,
};

use dotenv::dotenv;
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize, Clone, Default)]
pub struct Config {
    pub db_path: String,
    pub db_migrations_path: String,
    pub farcaster_priv_key: String,
    pub optimism_l2_rpc_url: String,
    pub chain_id: u32,
    pub id_registry_address: String,
    pub key_registry_address: String,
    pub storage_registry_address: String,
    pub abi_dir: String,
    pub indexer_interval: u64,
    pub bootstrap_addrs: Vec<String>,
}

impl Config {
    pub fn new() -> Self {

        dotenv().ok();
        // Load configuration from a TOML file and override with environment variables
        Figment::new()
            .merge(Toml::file("Config.toml"))
            .merge(Env::raw())
            .extract()
            .expect("configuration error")
    }

}
