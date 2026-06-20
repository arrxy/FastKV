use std::io::{Error, ErrorKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(Vec<u8>),
    Array(Vec<Value>),
    Null,
}

pub fn decode(data: &[u8]) -> Result<Value, Error> {
    if data.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "Empty data"));
    }
    decode_one(data).map(|(value, _)| value).map_err(|e| Error::new(ErrorKind::InvalidData, e))
}

fn decode_one(data: &[u8]) -> Result<(Value, usize), Error> {
    if data.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "Empty data"));
    }
    let (value, bytes_read) = match data[0] {
        b'+' => decode_simple_string(data),
        b'-' => decode_error(data),
        b':' => decode_integer(data),
        b'$' => decode_bulk_string(data),
        b'*' => decode_array(data),
        _ => return Err(Error::new(ErrorKind::InvalidData, "Invalid data")),
    }?;
    Ok((value, bytes_read))
}

fn decode_simple_string(data: &[u8]) -> Result<(Value, usize), Error> {
    let mut pos = 1;
    while pos < data.len() && data[pos] != b'\r' {
        pos += 1;
    }
    if pos >= data.len() {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid data"));
    }
    Ok((Value::SimpleString(String::from_utf8_lossy(&data[1..pos]).to_string()), pos + 2))
}

fn decode_error(data: &[u8]) -> Result<(Value, usize), Error> {
    decode_simple_string(data)
}

fn decode_integer(data: &[u8]) -> Result<(Value, usize), Error> {
    let mut pos: usize = 1;
    while pos < data.len() && data[pos] != b'\r' {
        pos += 1;
    }
    if pos >= data.len() {
        return Err(Error::new(ErrorKind::InvalidData, "Invalid data"));
    }
    let value: i64 = String::from_utf8_lossy(&data[1..pos]).parse().unwrap();
    Ok((Value::Integer(value), pos + 2))
}

fn decode_bulk_string(data: &[u8]) -> Result<(Value, usize), Error> {
    Ok((Value::Null, 0))
}

fn decode_array(data: &[u8]) -> Result<(Value, usize), Error> {
    Ok((Value::Null, 0))
}