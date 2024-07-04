use {
    ruc::*,
    serde::{Deserialize, Serialize},
    std::{fs::File, io::Read},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub http_port: u64,
    pub ws_port: u64,
    pub redis_url: Vec<String>,
    pub tendermint_url: String,
    pub chain_id: u32,
    pub gas_price: u64,
    pub postgres_uri: String,
}

impl Config {
    pub fn new(path: &str) -> Result<Self> {
        let mut file = File::open(path).c(d!())?;

        let mut str = String::new();
        file.read_to_string(&mut str).c(d!())?;

        let config: Config = toml::from_str(&str).c(d!())?;
        Ok(config)
    }
}
