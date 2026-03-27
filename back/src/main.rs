#![allow(warnings)]
use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Read, Write};
use std::env;
use std::iter::Iterator;
use std::sync::Arc;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use colored::*;

///# Function that log messages in a pattern.
/// # Arguments:
/// * ip: &Arc<String> - reference to a Arc pointer containing a String
/// * msg: String - message that will be logged
fn report(ip: &Arc<String>, msg: String) {
    println!("[{}@{}{}] {}", "SERVER".red(), ip.red(), ":1445".red(), msg.green());
}

///# Function that makes the handshake with proxy
///
/// # Arguments:
/// * max_clients: &i32 - max connections this server will hold
/// * server_ip: String - server IP
///
/// # Returns:
/// * Result<(), String> - returns nothing proxy received the register request or returns a string if proxy do not receive the register request.
fn register_with_balancer(max_clients: &i32, server_ip: String) -> Result<(), String> {

    let req_body = format!("{} : {}", max_clients, server_ip);

    match TcpStream::connect("127.0.0.1:2006") {
        Ok(mut stream ) => {
            let request = format!(
                "POST /server-register HTTP/1.1\r\n\
                Host: 127.0.0.1:2006\r\n\
                Content-Type: text/plain\r\n\
                Content-Length: {}\r\n\
                \r\n\
                {}",
                req_body.len(),
                req_body
            );

            stream.write(request.as_bytes()).unwrap();
            stream.flush().unwrap();

            let mut response_buffer = [0; 512];
            stream.read(&mut response_buffer).unwrap();
            let response = String::from_utf8_lossy(&response_buffer);

            if response.starts_with("HTTP/1.1 200 OK") {
                stream.shutdown(std::net::Shutdown::Both).unwrap();
                Ok(())
            } else {
                Err(format!("Handshake with proxy have failed. Proxy's Answer: {}", response))
            }
        },
        Err(_) => Err(format!("Connection with balancer have failed!"))
    }

}

///# Function that handles the connection with the proxy
///
/// # Arguments:
/// * mut stream: TcpStream - stream that holds the connection and the streams
/// * server_ip: Arc<String> - smart pointer that holds the server_ip
fn handle_connection(mut stream: TcpStream, server_ip: Arc<String>) -> std::io::Result<()> {
    let mut reader = BufReader::new(&mut stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    report(&server_ip, format!("Request received! {}", request_line));

    let response = "HTTP/1.1 200 OK\r\n\r\n";

    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
    stream.shutdown(std::net::Shutdown::Both);
    Ok(())
}

///# Main Function
///
/// Get IP from command execution args, set the listener, define the max_connections per server, keep trying to handshake with proxy
/// before start to processing requests.
fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 1 {
        eprintln!("No IP has been passed.");
        return;
    }

    let ip = Arc::new(args.get(1).unwrap().to_string());
    let ip_clone = Arc::clone(&ip);

    let listener = TcpListener::bind(format!("{}:1445", &ip_clone)).unwrap();

    let max_connections = 5;

    report(&ip_clone, format!("Listening at {}:1445", &ip_clone));

    report(&ip_clone, format!("Starting Handshake with {}", "Balancer@127.0.0.1:2006".bright_green()));

    while let Err(e) = register_with_balancer(&max_connections, String::from(format!("{}:1445", &ip_clone))) {
        report(&ip_clone, format!("{}", "Proxy is not receiving any requests!".red()));
        sleep(Duration::new(1, 0));
    }

    report(&ip_clone, format!("Handshake succeded!"));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let ip_clone = Arc::clone(&ip);
        thread::spawn(move || {
            handle_connection(stream, ip_clone).unwrap();
        });
    }
}
