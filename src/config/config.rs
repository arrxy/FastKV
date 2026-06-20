pub struct Config {
    port: u16,
    host: String,
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
        }
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_host(&self) -> &str {
        &self.host
    }
}
