use std::io::{self, ErrorKind};
use std::net::TcpStream;

use crate::core::cmd::RedisCommand;
use crate::core::eval::RedisState;
use crate::core::resp;
use crate::protocol::CommandProcessor;

pub struct RespCommandProcessor {
    state: RedisState,
}

impl RespCommandProcessor {
    pub fn new() -> Self {
        Self {
            state: RedisState::new(),
        }
    }
}

impl CommandProcessor for RespCommandProcessor {
    fn process(&mut self, data: &[u8], client_stream: &mut TcpStream) -> io::Result<()> {
        let commands = resp::decode_commands(data).map_err(|e| {
            io::Error::new(ErrorKind::InvalidData, e.to_string())
        })?;

        if commands.is_empty() {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "Empty Redis command",
            ));
        }

        for tokens in commands {
            let mut tokens = tokens.into_iter();
            let mut cmd_name = tokens.next().expect("checked non-empty above");
            cmd_name.make_ascii_uppercase();
            let args: Vec<String> = tokens.collect();
            let cmd = RedisCommand::new(cmd_name, args);
            self.state.eval_and_respond(&cmd, client_stream)?;
        }

        Ok(())
    }

    fn cleanup_expired_keys(&mut self) -> io::Result<()> {
        self.state.cleanup_expired_keys();
        Ok(())
    }
}
