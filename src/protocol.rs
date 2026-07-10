use std::io;

pub trait CommandProcessor {
    /// Parse and execute the complete commands in `data`, appending RESP
    /// responses to `out`. Returns how many bytes of `data` were consumed;
    /// a trailing partial command is left for the caller to retry with
    /// more bytes.
    fn process(&mut self, data: &[u8], out: &mut Vec<u8>) -> io::Result<usize>;
    fn cleanup_expired_keys(&mut self) -> io::Result<()>;
}
