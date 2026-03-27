#![allow(warnings)]

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, TcpListener, TcpStream};
use std::io::{BufRead, BufReader, Read, Write};
use std::str::FromStr;
use std::{io, thread};
use std::fmt::format;
use std::sync::{Arc, Mutex};
use colored::*;

///# Type that describes the Shared Hashmap of Ip Addresses that are wrapped in a Arc Mutex.
///
/// * HashMap<IpAddr, IpAddr> - The key is the clients IP and the value is the Server IP
type SharedIpList = Arc<Mutex<HashMap<IpAddr, IpAddr>>>;
///# Type that describes the Shared Hashmap of Server Capacities that are wrapped in a Arc Mutex.
///
/// * HashMap<IpAddr, (i32, i32)> - The key is the Server IP and the value is a table of i32, which the first element is the max capacity of the server and the second element is the current ammount of IP registered.
type SharedSpaceList = Arc<Mutex<HashMap<IpAddr, (i32, i32)>>>;


///# Struct that map a request most important fields.
///# Arguments:
/// * method: String - request's verb.
/// * uri: String - request's uri.
/// * host: String - request's sender IP.
/// * body: String - request's body.
/// * headers: Vec<String> - a vector containing all other request's headers.
struct Request {
    method: String,
    uri: String,
    host: String,
    body: String,
    headers: Vec<String>
}

///# Function that print patterned logs at console.
///
/// # Arguments
/// * msg: String - Message that will be logged.
fn report(msg: String) {
    println!("[{}@{}] {}", "BALANCER".blue(), "127.0.0.1:2006".blue(), msg.yellow());
}

///# Function that parses the request received.
///
/// # Arguments:
/// * mut stream: &TcpStream - mutable reference for the stream that holds the connection and request.
///
/// # Returns:
/// * returns Request - return a Request struct.
fn parse(mut stream: &TcpStream) -> Request {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).unwrap();
    if request_line.contains("HTTP") {
        let mut rl_items = request_line.split_whitespace();

        let method = rl_items.next().unwrap();
        let uri = rl_items.next().unwrap();

        let mut headers = Vec::new();
        let mut content_lengh = 0;

        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            if line == "\r\n" || line =="\n" {
                break;
            }

            if line.to_lowercase().starts_with("content-length:") {
                if let Some(value) = line.split(": ").nth(1) {
                    content_lengh = value.trim().parse::<usize>().unwrap_or(0);
                }
            }

            headers.push(line.trim().to_string())
        }


        let mut body = vec![0u8; content_lengh];
        if content_lengh > 0 {
            reader.read_exact(&mut body).unwrap();
        }


        let body_str = String::from_utf8_lossy(&body);

        Request {
            method: method.to_string(),
            headers: headers,
            body: body_str.to_string(),
            uri: uri.to_string(),
            host: "0.0.0.0:2006".to_string()
        }
    } else {
        Request {
            method: "GET".to_string(),
            headers: ["h1".to_string()].to_vec(),
            body: request_line,
            uri: "/".to_string(),
            host: "0.0.0.0:2006".to_string()
        }
    }

}

///# Function that selects the emptier server of the registered servers.
///
/// # Arguments:
/// * SpaceList: &SharedSpaceList - Arc Mutex List that contains a HashMap of server IP and its capabilites.
///
/// # Returns:
/// * Result<IpAddr, String> - If a server not full is found, returns the IpAddr of the server, else it will returns a String containg an error message.
fn distribute(SpaceList: &SharedSpaceList) -> Result<IpAddr, String> {
    let mut sharedSpace = SpaceList.lock().unwrap();


    let target = *sharedSpace.iter().min_by_key(|&(_ip, count)| count.1).unwrap().0;

    let values = sharedSpace.get(&target).unwrap();

    if (values.0 == values.1) {
        return Err("No server avaible!".to_string());
    }

    if let Some(value) = sharedSpace.get_mut(&target) {
        value.1 += 1;
    }

    Ok(target)

}

///# Function that handle the connection of new clients or servers in first hand.
///
/// # Arguments:
/// * mut stream: TcpStream - the stream that holds the connection and requests.
/// * IpList: SharedIpList - Arc Mutex HashMap that contains the clients Ip that are attatched to one of the servers.
/// * SpaceList: SharedSpaceList - Arc Mutex HashMap that contains the max and current capacity of each regustered servers.
fn handle_connection(mut stream: TcpStream, IpList: SharedIpList, SpaceList: SharedSpaceList) -> std::io::Result<()> {
    let request = parse(&stream);

    if request.method == "POST" && request.uri == "/server-register" {
        let mut body_parts = request.body.split_once(" : ").unwrap();
        let capacity: i32 = str::parse(body_parts.0.trim()).unwrap();
        let server_ip = IpAddr::from_str(body_parts.1.trim().split_once(":").unwrap().0).unwrap();

        report(format!("Server@{} is trying to sing up!", server_ip.to_string().red()));

        let mut sharedSpaces = SpaceList.lock().unwrap();

        if !sharedSpaces.contains_key(&server_ip) {
            report(format!("Registering Server@{}", server_ip.to_string()));
            sharedSpaces.insert(server_ip, (capacity, 0));
            report(format!("Showing avaiable server and spaces!"));
            let mut i = 0;
            for (k, v) in sharedSpaces.iter() {
                report(format!("Server#{} :: {} -> {}", i, k.to_string().green(), v.0.to_string().red()));
                i += 1;
            }
            report(format!("Server successfully registered!"));
        }
        drop(sharedSpaces);

        let response = "HTTP/1.1 200 OK\r\n\r\n";
        stream.write(response.as_bytes());
        stream.flush().unwrap();

    } else {
        let client_ip = stream.peer_addr()?.ip();

        let server_ip: IpAddr;

        report(format!("A client ({}) sent a request!", client_ip.to_string().red()));

        let mut sharedIps = IpList.lock().unwrap();

        if !sharedIps.contains_key(&client_ip) {
            report(format!("Attaching client to a server!"));
            server_ip = match distribute(&SpaceList) {
                Ok(ip) => ip,
                Err(_) => {
                    let error_response = format!(
                        "HTTP/1.1 503 BAD GATEWAY\r\n\r\n"
                    );

                    stream.write(error_response.as_bytes()).unwrap();
                    stream.flush().unwrap();
                    stream.shutdown(std::net::Shutdown::Both);
                    drop(sharedIps);
                    panic!();
                }

            };
            report(format!("Client@{} was attached to Server@{}", client_ip.to_string().red(), server_ip.to_string().blue()));
            sharedIps.insert(client_ip, server_ip.clone());
        } else {
            server_ip = *sharedIps.get(&client_ip).unwrap();
            report(format!("Client is already attached to Server@{}", server_ip.to_string().blue()));
        }

        report(format!("Forwarding client's request!"));

        proxy_forward(request, stream, server_ip);
    }

    Ok(())
}

///# Function that forwards client request to its attathed server.
///
/// # Arguments:
/// * request: Request - the request struct
/// * mut stream: TcpStream - the stream that holds the connection.
/// * server_ip: IpAddr - the client attatched IP server.
fn proxy_forward(request: Request, mut stream: TcpStream, server_ip: IpAddr) {
    let mut server_stream = TcpStream::connect(format!("{server_ip}:1445")).unwrap();
    let mut server_request: String = "GET / HTTP/1.1\r\n\r\n".to_string();

    if request.method == "GET" {
        server_request = format!(
          "{} {} HTTP/1.1\r\n\
          Server-Ip: {}\r\n
          Host: {}\r\n\
          \r\n",
            request.method,
            server_ip,
            request.uri,
            request.host
        );
    } else if request.method == "POST" {
        server_request = format!(
            "{} {} HTTP/1.1\r\n\
            Server-Ip: {}\r\n
            Host: {}\r\n\
            Content-Length: {}\r\n\
            Content-Type: {}\r\n\
            \r\n\
            {}",
            request.method,
            server_ip,
            request.uri,
            request.host,
            request.body.len(),
            "plain/text",
            request.body
        );

    }

    report(format!("Sending request to Server@{}", server_ip.to_string().yellow()));

    server_stream.write(server_request.as_bytes()).unwrap();
    server_stream.flush().unwrap();

    io::copy(&mut server_stream, &mut stream).unwrap();

    report(format!("Sending back response to Client@{}", stream.peer_addr().unwrap().ip().to_string().red()));

    server_stream.shutdown(std::net::Shutdown::Both).unwrap();
}



/// # Main Function
///
/// Creates the listener at the provided IP, declares both of Arc Mutex for data manipulations inside the threads.
fn main() {
    let listener = TcpListener::bind("127.0.0.1:2006").unwrap();

    report(format!("Listening at 127.0.0.1:2006!"));

    let shared_IpList: SharedIpList = Arc::new(Mutex::new(HashMap::new()));
    let shared_IpSpace: SharedSpaceList = Arc::new(Mutex::new(HashMap::new()));

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let IpList_clone = Arc::clone(&shared_IpList);
        let SpaceList_clone = Arc::clone(&shared_IpSpace);
        thread::spawn(move || {
            handle_connection(stream, IpList_clone, SpaceList_clone).unwrap();
        });
    }
}
