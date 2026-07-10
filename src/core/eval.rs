use std::{
    io::Write,
    net::TcpStream,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use chaintable::{Dict, EvictionPolicy};

use crate::{
    config::config::Config, core::{
        cmd::RedisCommand, resp::{Value, encode},
    },
};

pub struct RedisValue {
    pub value: Value,
    pub expires_at: i64,
}

pub struct RedisState {
    pub data: Dict<RedisValue>,
    pub max_keys: usize,
    pub eviction_policy: Option<EvictionPolicy>,
}

impl RedisState {
    pub fn new() -> Self {
        let config = Config::new();
        Self {
            data: Dict::new(),
            max_keys: config.get_max_keys(),
            eviction_policy: config.get_eviction_policy(),
        }
    }

    pub fn cleanup_expired_keys(&mut self) {
        let now = self.now_millis() as u64;
        let start = Instant::now();
        const TIME_BUDGET_MS: u128 = 25;
        let mut rng = rand::rng();
        loop {
            let slots = self.data.sample_volatile_slots(20, &mut rng);
            if slots.is_empty() {
                break;
            }
            let sampled = slots.len();
            let expired_keys: Vec<String> = slots
                .into_iter()
                .filter_map(|slot| self.data.entry_ref(slot))
                .filter(|e| e.expires_at.is_some_and(|t| t <= now))
                .map(|e| e.key.to_string())
                .collect();
            let expired_ratio = expired_keys.len() as f64 / sampled as f64;
            for key in &expired_keys {
                self.data.remove(key);
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
        let (key, value, expires_at) = match self.validate_and_get_set_args(args, client_stream) {
            Ok(v) => v,
            Err(_) => return Ok(()),
        };

        let mut rng = rand::rng();
        while self.data.len() >= self.max_keys && !self.data.contains_key(&key) {
            let evicted = self
                .eviction_policy
                .and_then(|policy| self.data.evict(policy, &mut rng));
            if evicted.is_none() {
                self.send_error(
                    "OOM command not allowed when used memory > 'maxmemory'.",
                    client_stream,
                )?;
                return Ok(());
            }
        }

        let now = self.now_millis();
        self.data.insert_with_meta(
            key.into(),
            RedisValue {
                value,
                expires_at: expires_at.unwrap_or(-1),
            },
            expires_at.map(|t| t as u64),
            Some(now as u64),
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
        let key = &args[0];
        let now = self.now_millis();
        if self
            .data
            .get(key)
            .is_some_and(|v| v.expires_at != -1 && v.expires_at < now)
        {
            self.data.remove(key);
        }
        let encoded = match self.data.get(key) {
            Some(v) => encode(&v.value)?,
            None => {
                client_stream.write_all(&encode(&Value::Null)?)?;
                return Ok(());
            }
        };
        self.data.touch(key, Some(now as u64));
        client_stream.write_all(&encoded)?;
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
            self.data.remove(&key);
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
            if self.data.remove(key).is_some() {
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
            }

            None => {
                client_stream.write_all(&encode(&Value::Integer(0))?)?;
                return Ok(());
            }
        }
        self.data.set_expiry(key, Some(expires_at as u64));
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
        assert_eq!(state.data.volatile_len(), 1, "only k1 has expiry");

        std::thread::sleep(std::time::Duration::from_millis(5));
        state.cleanup_expired_keys();

        assert!(
            state.data.get("k1").is_none(),
            "expired key should be swept"
        );
        assert_eq!(
            state.data.volatile_len(),
            0,
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
        assert_eq!(state.data.volatile_len(), 1);
        state.eval_set(&argv(&["k", "v2"]), &mut s).unwrap(); // overwrite, no TTL
        assert_eq!(
            state.data.volatile_len(),
            0,
            "stale TTL index must clear"
        );
    }
}
