use std::{error::Error, fmt};
use RESP2Command::Ping;
use crate::parser::command::RESP2Command;
use crate::parser::command::RESP2Command::Echo;

use crate::parser::read_until_crlf;
use crate::parser::resp_type::Type::BulkString;

// https://redis.io/docs/reference/protocol-spec/#simple-strings
// https://redis.io/docs/reference/protocol-spec/#simple-errors
// https://redis.io/docs/reference/protocol-spec/#integers
// https://redis.io/docs/reference/protocol-spec/#bulk-strings
// https://redis.io/docs/reference/protocol-spec/#arrays
#[repr(u8)]
#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    SimpleString(String),
    SimpleError(String),
    Integer(i64),
    BulkString(String),
    Array { content: Vec<Type>, count: u8 },
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParserError {
    Utf8(std::string::FromUtf8Error),
    Parse(std::num::ParseIntError),
    Read,
    IncorrectNumberOfElements,
    CRLF,
    CorruptedData,
    ExpectedCRLF,
    UnexpectedType,
    NotSupportedType(String),
    ExpectedIndex,
}

impl Error for ParserError {}

impl From<std::num::ParseIntError> for ParserError {
    fn from(value: std::num::ParseIntError) -> Self {
        ParserError::Parse(value)
    }
}

impl From<std::string::FromUtf8Error> for ParserError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        ParserError::Utf8(value)
    }
}


pub fn decode(payload: &[u8], cursor: Option<usize>) -> Result<(Type, Option<usize>), ParserError> {
    let (element, new_cursor) = read_until_crlf(payload, cursor).ok_or(ParserError::Read)?;
    match element[0] {
        //b'+' => Ok(Type::SimpleStrings),
        //b'-' => Ok(Type::SimpleErrors),
        //b':' => Ok(Type::Integers),
        b'$' => {
            let res = handle_bulk_string(&payload, cursor);
            match res {
                Ok(t) => Ok(t),
                Err(e) => Err(e)
            }
        }
        b'*' => {
            let res = handle_array(&payload, cursor);
            match res {
                Ok(t) => Ok(t),
                Err(e) => Err(e)
            }
        }
        _ => Err(ParserError::NotSupportedType(element[0].to_string())),
    }
}

pub fn encode(data: &RESP2Command) -> Vec<u8> {
    match data {
        Ping(s) => {
            format!("+{}\r\n", s).into_bytes()
        },
        Echo(s) => {
            let l = s.len();
            format!("${}\r\n{}\r\n", l, s).into_bytes()
        },
        _ => format!("-ERR\r\n").into_bytes(),
    }
}

impl fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported type")
    }
}

// 2\n\r $4\n\r echo \n\r
fn handle_array(data: &[u8], cursor: Option<usize>) -> Result<(Type, Option<usize>), ParserError> {
    let mut start_index: usize;
    let (s, mut i) = read_until_crlf(data, cursor).ok_or(ParserError::Read)?;

    let array_length = String::from_utf8_lossy(&s[1..]).parse::<usize>()?;

    start_index = i.ok_or(ParserError::ExpectedIndex)?;

    let mut content = Vec::with_capacity(array_length);

    for i in 0..array_length
    {
        let t = decode(data, Some(start_index))?;
        if let Some(new_index) = t.1 {
            start_index = new_index; // if index >= len
        }
        content.push(t.0);
    };

    Ok((Type::Array {
        content,
        count: array_length as u8,
    }, Some(start_index)))
}


/// handles bulk string with naive implementation
///
/// # Arguments
///
/// * `data`:
/// * `cursor`:
///
/// returns: Result<(Type, Option<usize>), ParserError>
///
/// # Examples
///
/// ```
///
/// ```
fn handle_bulk_string(data: &[u8], cursor: Option<usize>) -> Result<(Type, Option<usize>), ParserError> {
    let (s, i) = read_until_crlf(data, cursor).ok_or(ParserError::CRLF)?;

    let payload_beginning_index = i.ok_or(ParserError::ExpectedIndex)?;
    
    let expected_count = String::from_utf8(Vec::from(&s[1..]))?.parse::<usize>()?;

    if expected_count == 0 {
        if has_next(data, payload_beginning_index) && has_crlf_at(data, payload_beginning_index) {
            return Ok((BulkString("".to_string()), Some(payload_beginning_index + 2)));
        }
        if is_last(data, payload_beginning_index) && has_crlf_at(data, payload_beginning_index) {
            return Ok((BulkString("".to_string()), None));
        }
        return Err(ParserError::CorruptedData);
    }

    let next_read_index = payload_beginning_index + expected_count;

    if has_next(data, next_read_index) && has_crlf_at(data, next_read_index) {
        let s = String::from_utf8(Vec::from(&data[payload_beginning_index..next_read_index]))?;
        return Ok((BulkString(s), Some(next_read_index + 2)));
    }
    if is_last(data, next_read_index) && has_crlf_at(data, next_read_index) {
        let s = String::from_utf8(Vec::from(&data[payload_beginning_index..next_read_index]))?;
        return Ok((BulkString(s), None));
    }
    Err(ParserError::CorruptedData)
}

fn has_crlf_at(data: &[u8], index: usize) -> bool {
    data.get(index) == Some(&b'\r') && data.get(index + 1) == Some(&b'\n')
}

fn has_next(data: &[u8], index: usize) -> bool {
    data.len() > index + 2
}

fn is_last(data: &[u8], index: usize) -> bool {
    data.len() > index + 1
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use crate::parser::resp_type::{decode, handle_array, handle_bulk_string, Type};

    #[test]
    fn tc_1_basic_handle_bulk_string() {
        let t1 = b"$4\r\necho\r\n";
        let result = handle_bulk_string(t1, None).unwrap();
        if let (Type::BulkString(x), cursor) = result {
            assert_eq!(x, "echo");
        }
    }

    #[test]
    fn tc_2_basic_handle_bulk_string() {
        let t1 = b"$4\r\necho\r\n423423";
        let result = handle_bulk_string(t1, None).unwrap();
        if let (Type::BulkString(x), cursor) = result {
            assert_eq!(x, "echo");
        }
    }
    #[test]
    fn tc_3_empty_handle_bulk_string() {
        let t1 = b"$0\r\n\r\n";
        let result = handle_bulk_string(t1, None).unwrap();
        if let (Type::BulkString(x), cursor) = result {
            assert_eq!(x, "");
        }
    }

    #[test]
    fn tc_4_empty_handle_bulk_string_with_tail() {
        let t1 = b"$0\r\n\r\n4444424";
        let result = handle_bulk_string(t1, None).unwrap();
        if let (Type::BulkString(x), cursor) = result {
            assert_eq!(x, "");
        }
    }

    #[test]
    fn tc_5_basic_handle_array() {
        let t1 = b"*2\r\n$4\r\necho\r\n$5\r\necho2\r\n";
        //let              b" *2 \r\n $4 \r\n echo \r\n $5 \r\n echo2 \r\n ";
        let result = decode(t1, None).unwrap();
        if let Type::Array { content, count } = result.0 {
            assert_eq!(count, 2)
        }
    }

    #[test]
    fn tc_6_nested_handle_array() {
        let t1 = b"*2\r\n*2\r\n$4\r\necho\r\n$5\r\necho2\r\n$5\r\necho2\r\n";
        let result = decode(t1, None).unwrap();
        if let Type::Array { content: inner_content, count: inner_count } = result.0 {
            assert_eq!(inner_count, 2);
            for entry in inner_content {
                if let Type::Array { content, count } = entry {
                    assert_eq!(count, 2);
                    assert_eq!(*content.first().unwrap(), Type::BulkString("echo".to_string()));
                    assert_eq!(*content.get(1).unwrap(), Type::BulkString("echo2".to_string()));
                }
            }
        }
    }
}