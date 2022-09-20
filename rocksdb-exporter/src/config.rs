use {
    ruc::*,
    serde::{Deserialize, Serialize},
    std::{fs::File, io::Read},
};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub state_db_path: String,
    pub history_db_path: String,
    pub redis_url: Vec<String>,
    pub clear: bool,
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
