// Uncomment this block to pass the first stage
// use std::net::TcpListener;

mod parser;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{io, thread};
use std::time::Duration;
use crate::parser::command::RESP2Command;
use crate::parser::resp_type::{decode, encode, Type};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                thread::spawn(|| {
                    handle(_stream);
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

// need rework so it does not block, timeout temp fix
fn handle(mut s: TcpStream) {
    s.set_read_timeout(Some(Duration::new(1, 0))).expect("Failed to set read timeout");
    let mut buffer = [0; 1024];
    let mut v = Vec::new();
    loop {
        match s.read(&mut buffer) {
            Ok(0) => break, 
            Ok(n) => v.extend_from_slice(&buffer[..n]), 
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                
                break; 
            },
            Err(e) => {
                eprintln!("Failed to read from the stream: {}", e);
                return;
            }
        }
    }
     let res = decode(&v, None).unwrap().0;
     let cmd = RESP2Command::try_from(res).unwrap();
     s.write_all(encode(&cmd).as_slice()).expect("error writing to stream");

}

