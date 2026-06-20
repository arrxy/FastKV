use crate::config::config::Config;
use std::io::{ErrorKind, prelude::*};
use std::net::{TcpListener, TcpStream};

pub fn run_sync_tcp_server() {
    let config: Config = Config::new();
    let mut con_clients: u64 = 0;
    println!(
        "Server running on {}:{}",
        config.get_host(),
        config.get_port()
    );
    let listener: TcpListener =
        TcpListener::bind(format!("{}:{}", config.get_host(), config.get_port()))
            .expect("Failed to bind to address");
    loop {
        let mut client_stream: TcpStream = match listener.accept() {
            Ok((stream, address)) => {
                con_clients += 1;
                println!(
                    "New connection from {}:{}, concurrent connections: {}",
                    address.ip(),
                    address.port(),
                    con_clients
                );
                stream
            }
            Err(e) => {
                println!("Error accepting connection: {}", e);
                break;
            }
        };
        loop {
            let cmd: String = match read_client_command(&mut client_stream) {
                Ok(cmd) => cmd,
                Err(e) => {
                    con_clients -= 1;
                    if let Err(e) = client_stream.shutdown(std::net::Shutdown::Both) {
                        println!("Shutdown failed/ignored: {}", e);
                    }
                    println!("Error reading from client: {}, concurrent connections: {}", e, con_clients);
                    break;
                }
            };
            match respond_to_client(&cmd, &mut client_stream) {
                Ok(_) => {},
                Err(e) => {
                    println!("Error responding to client: {}", e);
                    con_clients -= 1;
                    client_stream.shutdown(std::net::Shutdown::Both).unwrap();
                    println!("Error responding to client: {}, closed connection, concurrent connections: {}", e, con_clients);
                    println!("Client closed connection");
                    break;
                }
            }
        }
    }
}

fn read_client_command(client_stream: &mut TcpStream) -> Result<String, std::io::Error> {
    let mut buffer: [u8; 1024] = [0; 1024];
    let n: usize = match client_stream.read(&mut buffer) {
        Ok(n) => n,
        Err(e) => return Err(e),
    };
    if n == 0 {
        return Err(std::io::Error::new(ErrorKind::UnexpectedEof, "Client closed connection"));
    }
    let cmd: String = String::from_utf8_lossy(&buffer[..n]).to_string();
    Ok(cmd)
}

fn respond_to_client(cmd: &str, client_stream: &mut TcpStream) -> Result<(), std::io::Error> {
    match client_stream.write_all(cmd.as_bytes()) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}