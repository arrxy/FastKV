use rand::RngExt;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::TcpStream,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use crate::{
    config::config::Config, core::{
        cmd::RedisCommand, evict, resp::{Value, encode},
    },
};

pub struct RedisValue {
    pub value: Value,
    pub expires_at: i64,
    pub access_count: u64,
    pub last_accessed_at: i64,
}

#[derive(Copy, Clone, PartialEq)]
pub enum EvictionPolicy {
    NoEviction,
    AllKeysRandom,
    VolatileRandom,
    AllKeysLru,
    VolatileLru,
    AllKeysLfu,
    VolatileLfu,
    VolatileTtl,
}

pub struct RedisState {
    data: HashMap<String, RedisValue>,
    volatile_keys: HashSet<String>,
    pub max_keys: usize,
    pub eviction_sample_size: usize,
    pub eviction_policy: EvictionPolicy,
}

impl RedisState {
    pub fn new() -> Self {
        let config = Config::new();
        Self {
            data: HashMap::new(),
            volatile_keys: HashSet::new(),
            max_keys: config.get_max_keys(),
            eviction_sample_size: config.get_eviction_sample_size(),
            eviction_policy: config.get_eviction_policy(),
        }
    }

    fn sample_iter<T>(iter: impl Iterator<Item = T>, n: usize) -> Vec<T> {
        let mut rng = rand::rng();
        let mut reservoir: Vec<T> = Vec::with_capacity(n);
        for (i, item) in iter.enumerate() {
            if i < n {
                reservoir.push(item);
            } else {
                let j = rng.random_range(0..=i);
                if j < n {
                    reservoir[j] = item;
                }
            }
        }
        reservoir
    }

    fn remove_key(&mut self, key: &str) -> Option<RedisValue> {
        self.volatile_keys.remove(key);
        self.data.remove(key)
    }

    pub fn cleanup_expired_keys(&mut self) {
        let now = self.now_millis();
        let start = Instant::now();
        const TIME_BUDGET_MS: u128 = 25;
        loop {
            let sample: Vec<String> = Self::sample_iter(self.volatile_keys.iter().cloned(), 20);
            if sample.is_empty() {
                break;
            }
            let expired_keys: Vec<String> = sample
                .iter()
                .filter(|k| self.data.get(*k).map_or(false, |v| v.expires_at <= now))
                .cloned()
                .collect();
            let expired_ratio = expired_keys.len() as f64 / sample.len() as f64;
            for key in &expired_keys {
                self.remove_key(key);
            }

            if expired_ratio <= 0.25 || start.elapsed().as_millis() >= TIME_BUDGET_MS {
                break;
            }
        }
    }

    pub fn eval_and_respond(
        &mut self,
        cmd: &RedisCommand,
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        match cmd.cmd.as_str() {
            "PING" => self.eval_ping(&cmd.args, client_stream),
            "ECHO" => self.eval_echo(&cmd.args, client_stream),
            "SET" => self.eval_set(&cmd.args, client_stream),
            "GET" => self.eval_get(&cmd.args, client_stream),
            "TTL" => self.eval_ttl(&cmd.args, client_stream),
            "DEL" => self.eval_del(&cmd.args, client_stream),
            "EXPIRE" => self.eval_expire(&cmd.args, client_stream),
            _ => self.eval_ping(&cmd.args, client_stream),
        }
    }

    fn eval_ping(
        &self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        if args.len() >= 2 {
            let encoded = encode(&Value::Error(
                "ERR wrong number of arguments for 'ping' command".to_string(),
            ))?;
            client_stream.write_all(&encoded)?;
            return Ok(());
        }
        let response = if args.is_empty() {
            Value::SimpleString("PONG".to_string())
        } else {
            Value::BulkString(args[0].clone().into_bytes())
        };
        client_stream.write_all(&encode(&response)?)?;
        Ok(())
    }

    fn eval_echo(
        &self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        if args.len() >= 2 || args.is_empty() {
            let encoded = encode(&Value::Error(
                "ERR wrong number of arguments for 'echo' command".to_string(),
            ))?;
            client_stream.write_all(&encoded)?;
            return Ok(());
        }
        let response = Value::BulkString(args[0].clone().into_bytes());
        client_stream.write_all(&encode(&response)?)?;
        Ok(())
    }

    fn now_millis(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn send_error(&self, error: &str, client_stream: &mut TcpStream) -> Result<(), std::io::Error> {
        let encoded = encode(&Value::Error(error.to_string()))?;
        client_stream.write_all(&encoded)?;
        Ok(())
    }

    fn reject<T>(&self, error: &str, client_stream: &mut TcpStream) -> Result<T, std::io::Error> {
        self.send_error(error, client_stream)?;
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid input",
        ))
    }

    fn validate_and_get_set_args(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(String, Value, Option<i64>), std::io::Error> {
        if args.len() != 2 && args.len() != 4 {
            return self.reject(
                "ERR wrong number of arguments for 'set' command",
                client_stream,
            );
        }

        let mut expires_at: Option<i64> = None;
        if args.len() == 4 {
            let unit_ms: i64 = match args[2].to_uppercase().as_str() {
                "EX" => 1000,
                "PX" => 1,
                _ => return self.reject("ERR syntax error", client_stream),
            };
            expires_at = Some(self.parse_expiry(&args[3], unit_ms, client_stream)?);
        }

        let key = args[0].clone();
        let value = Value::BulkString(args[1].clone().into_bytes());
        Ok((key, value, expires_at))
    }

    fn parse_expiry(
        &self,
        amount: &str,
        unit_ms: i64,
        client_stream: &mut TcpStream,
    ) -> Result<i64, std::io::Error> {
        let amount: i64 = match amount.parse() {
            Ok(n) => n,
            Err(_) => {
                return self.reject("ERR value is not an integer or out of range", client_stream);
            }
        };
        if amount <= 0 {
            return self.reject("ERR invalid expire time in 'set' command", client_stream);
        }
        match amount
            .checked_mul(unit_ms)
            .and_then(|ttl_ms| self.now_millis().checked_add(ttl_ms))
        {
            Some(expires_at) => Ok(expires_at),
            None => self.reject("ERR value is not an integer or out of range", client_stream),
        }
    }

    fn eval_set(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        while self.data.len() >= self.max_keys && self.eviction_policy != EvictionPolicy::NoEviction {
            let before = self.data.len();
            evict::evict(self)?;
            let after = self.data.len();
            if after >= before {
                self.send_error("OOM command not allowed when used memory > 'maxmemory'.", client_stream)?;
                return Ok(());
            }
        }
        let (key, value, expires_at) = match self.validate_and_get_set_args(args, client_stream) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        match expires_at {
            Some(_) => {
                self.volatile_keys.insert(key.clone());
            }
            None => {
                self.volatile_keys.remove(&key);
            }
        }
        self.data.insert(
            key,
            RedisValue {
                value,
                expires_at: expires_at.unwrap_or(-1),
                access_count: 0,
                last_accessed_at: self.now_millis(),
            },
        );

        let response = Value::SimpleString("OK".to_string());
        client_stream.write_all(&encode(&response)?)?;
        Ok(())
    }

    fn eval_get(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        if args.len() != 1 {
            self.send_error(
                "ERR wrong number of arguments for 'get' command",
                client_stream,
            )?;
            return Ok(());
        }
        let key = args[0].clone();
        let value = match self.data.get(&key) {
            Some(v) => v,
            None => {
                client_stream.write_all(&encode(&Value::Null)?)?;
                return Ok(());
            }
        };
        if value.expires_at < self.now_millis() && value.expires_at != -1 {
            self.remove_key(&key);
            client_stream.write_all(&encode(&Value::Null)?)?;
            return Ok(());
        }

        client_stream.write_all(&encode(&value.value)?)?;
        Ok(())
    }

    fn eval_ttl(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        let key = args[0].clone();
        let value = match self.data.get(&key) {
            Some(v) => v,
            None => {
                client_stream.write_all(&encode(&Value::Integer(-2))?)?;
                return Ok(());
            }
        };
        if value.expires_at == -1 {
            client_stream.write_all(&encode(&Value::Integer(-1))?)?;
            return Ok(());
        }
        if value.expires_at < self.now_millis() {
            self.remove_key(&key);
            client_stream.write_all(&encode(&Value::Integer(-2))?)?;
            return Ok(());
        }
        let response_in_seconds = Value::Integer((value.expires_at - self.now_millis()) / 1000);
        client_stream.write_all(&encode(&response_in_seconds)?)?;
        Ok(())
    }

    fn eval_del(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        if args.len() == 0 {
            self.send_error(
                "ERR wrong number of arguments for 'del' command",
                client_stream,
            )?;
            return Ok(());
        }
        let mut deleted_count = 0;
        for key in args {
            if self.remove_key(key).is_some() {
                deleted_count += 1;
            }
        }
        let response = Value::Integer(deleted_count);
        client_stream.write_all(&encode(&response)?)?;
        Ok(())
    }

    fn eval_expire(
        &mut self,
        args: &[String],
        client_stream: &mut TcpStream,
    ) -> Result<(), std::io::Error> {
        if args.len() != 2 {
            self.send_error(
                "ERR wrong number of arguments for 'expire' command",
                client_stream,
            )?;
            return Ok(());
        };
        let key = &args[0];
        let expires_at = match self.parse_expiry(&args[1], 1000, client_stream) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };
        match self.data.get_mut(key) {
            Some(v) => {
                v.expires_at = expires_at;
                self.volatile_keys.insert(key.clone());
            }

            None => {
                client_stream.write_all(&encode(&Value::Integer(0))?)?;
                return Ok(());
            }
        }
        client_stream.write_all(&encode(&Value::Integer(1))?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    // A connected loopback stream; the server side is returned so the
    // connection isn't reset and small writes don't fail.
    fn loopback() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let client = TcpStream::connect(addr).unwrap();
        let (server, _) = listener.accept().unwrap();
        (client, server)
    }

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn cleanup_removes_only_expired_volatile_keys() {
        let mut state = RedisState::new();
        let (mut s, _server) = loopback();

        state
            .eval_set(&argv(&["k1", "v", "PX", "1"]), &mut s)
            .unwrap(); // volatile, expires fast
        state.eval_set(&argv(&["k2", "v"]), &mut s).unwrap(); // no TTL
        assert!(state.volatile_keys.contains("k1"));
        assert!(!state.volatile_keys.contains("k2"));

        std::thread::sleep(std::time::Duration::from_millis(5));
        state.cleanup_expired_keys();

        assert!(
            state.data.get("k1").is_none(),
            "expired key should be swept"
        );
        assert!(
            !state.volatile_keys.contains("k1"),
            "index must drop swept key"
        );
        assert!(state.data.get("k2").is_some(), "non-volatile key untouched");
    }

    #[test]
    fn overwriting_volatile_key_without_ttl_clears_index() {
        let mut state = RedisState::new();
        let (mut s, _server) = loopback();

        state
            .eval_set(&argv(&["k", "v", "EX", "100"]), &mut s)
            .unwrap();
        assert!(state.volatile_keys.contains("k"));
        state.eval_set(&argv(&["k", "v2"]), &mut s).unwrap(); // overwrite, no TTL
        assert!(
            !state.volatile_keys.contains("k"),
            "stale TTL index must clear"
        );
    }
}
