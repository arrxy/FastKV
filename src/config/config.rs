use crate::core::eval::EvictionPolicy;

pub struct Config {
    port: u16,
    host: String,
    cleanup_interval: u128,
    max_keys: usize,
    eviction_sample_size: usize,
    eviction_policy: EvictionPolicy,
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
            max_keys: std::env::var("MAX_KEYS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap(),
            eviction_sample_size: std::env::var("EVICTION_SAMPLE_SIZE")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .unwrap(),
            eviction_policy: match std::env::var("EVICTION_POLICY")
                .unwrap_or_else(|_| "NoEviction".to_string())
                .as_str()
            {
                "NoEviction" => EvictionPolicy::NoEviction,
                "AllKeysRandom" => EvictionPolicy::AllKeysRandom,
                "VolatileRandom" => EvictionPolicy::VolatileRandom,
                "AllKeysLru" => EvictionPolicy::AllKeysLru,
                "VolatileLru" => EvictionPolicy::VolatileLru,
                "AllKeysLfu" => EvictionPolicy::AllKeysLfu,
                "VolatileLfu" => EvictionPolicy::VolatileLfu,
                "VolatileTtl" => EvictionPolicy::VolatileTtl,
                _ => EvictionPolicy::NoEviction,
            },
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
