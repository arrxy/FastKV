use chaintable::EvictionPolicy;

pub struct Config {
    port: u16,
    host: String,
    cleanup_interval: u128,
    max_keys: usize,
    eviction_policy: Option<EvictionPolicy>,
}

impl Config {
    pub fn new() -> Self {
        Config::from_env()
    }
}

impl Config {
    fn from_env() -> Self {
        let eviction_sample_size = std::env::var("EVICTION_SAMPLE_SIZE")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap();
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
            eviction_policy: match std::env::var("EVICTION_POLICY")
                .unwrap_or_else(|_| "NoEviction".to_string())
                .as_str()
            {
                "AllKeysRandom" => Some(EvictionPolicy::AllkeysRandom),
                "VolatileRandom" => Some(EvictionPolicy::VolatileRandom),
                "AllKeysLru" => Some(EvictionPolicy::AllkeysLru { sample_size: eviction_sample_size }),
                "VolatileLru" => Some(EvictionPolicy::VolatileLru { sample_size: eviction_sample_size }),
                "AllKeysLfu" => Some(EvictionPolicy::AllkeysLfu { sample_size: eviction_sample_size }),
                "VolatileLfu" => Some(EvictionPolicy::VolatileLfu { sample_size: eviction_sample_size }),
                _ => None, // NoEviction
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

    pub fn get_max_keys(&self) -> usize {
        self.max_keys
    }

    pub fn get_eviction_policy(&self) -> Option<EvictionPolicy> {
        self.eviction_policy
    }
}
