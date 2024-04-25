pub mod command;
pub mod resp_type;

use crate::parser::resp_type::{decode, Type};



/// Reads buffer until encounters ` \r\n ` , returns additionally start index for next read
///
/// # Arguments 
///
/// * `buffer`: 
///
/// returns: Option<(&[u8], usize)>
fn read_until_crlf(buffer: &[u8], cursor: Option<usize>) -> Option<(&[u8], Option<usize>)> {
    let start_index = cursor.unwrap_or(0);
    for i in start_index + 1..buffer.len() {
        if buffer[i - 1] == b'\r' && buffer[i] == b'\n' {
            if i == buffer.len() - 1 {
                return Some((&buffer[start_index..(i - 1)], None));
            }
            return Some((&buffer[start_index..(i - 1)], Some(i + 1)));
        }
    }
    None
}
