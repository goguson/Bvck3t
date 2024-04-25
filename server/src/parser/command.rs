use crate::parser::resp_type::{ParserError, Type};
use std::{error::Error, fmt};
use log::error;

// Docs
// https://redis.io/docs/reference/protocol-spec/
// https://redis.io/docs/reference/protocol-spec/#bulk-strings

pub enum RESP2Command {
    Echo(String),
    Ping(String)
}

impl RESP2Command {
    pub fn to_string(&self) -> &str{
        match &self {
            RESP2Command::Echo(s) => s.as_str(),
            RESP2Command::Ping(s) => s.as_str()
        }
    }
}

impl TryFrom<Type> for RESP2Command {
    type Error = ParserError;
    fn try_from(value: Type) -> Result<Self, Self::Error> {
        match value {
            Type::Array { content, count } => {
                if let Some(&Type::BulkString(ref cmd)) = content.first() {
                    return match cmd.to_lowercase().as_str() {
                        "echo" => return handle_echo(&content[1..]),
                        "ping" => return Ok(RESP2Command::Ping("PONG".to_string())),
                        _ => Err(ParserError::ExpectedIndex)
                    };
                }
                return Err(ParserError::ExpectedIndex);
            }
            _ => Err(ParserError::ExpectedIndex)
        }
    }
}

pub fn handle_echo(data: &[Type]) -> Result<RESP2Command, ParserError> {
    if data.len() == 0 || data.len() > 1 {
        return Err(ParserError::IncorrectNumberOfElements);
    }
    let e = data.get(0).ok_or(ParserError::CorruptedData)?;
    match e {
        Type::BulkString(msg) => Ok(RESP2Command::Echo(msg.clone())),
        _ => Err(ParserError::UnexpectedType)
    }
}