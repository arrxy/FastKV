pub struct Config {
    port: u16,
    host: String,
    cleanup_interval: u128,
}

impl Config {
    pub fn new() -> Self {
        Config::from_env()
    }
}

impl Config {
    fn from_env() -> Self {
        Self {
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "9736".to_string())
                .parse()
                .unwrap(),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            cleanup_interval: std::env::var("CLEANUP_INTERVAL")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap(),
        }
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_host(&self) -> &str {
        &self.host
    }

    pub fn get_cleanup_interval(&self) -> u128 {
        self.cleanup_interval
    }
}
