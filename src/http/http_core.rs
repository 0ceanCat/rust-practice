use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use crate::utils::json::{DataType, JsonParser};

#[derive(Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl HttpMethod {
    fn from_str(str: &str) -> Self {
        let lowercase = str.trim().to_lowercase();
        match lowercase.as_str() {
            "get" => HttpMethod::GET,
            "post" => HttpMethod::POST,
            "put" => HttpMethod::PUT,
            "delete" => HttpMethod::DELETE,
            "patch" => HttpMethod::PATCH,
            _ => { panic!("Unknown method detected") }
        }
    }
}

enum HttpStatus {
    OK = 200,
    BadRequest = 400,
    Forbidden = 401,
    NotFound = 404,
    InternalError = 500,
}

pub(crate) struct HttpRequest {
    version: String,
    method: HttpMethod,
    headers: HashMap<String, String>,
    body: HashMap<String, DataType>,
}

impl HttpRequest {
    fn new(mut stream: &TcpStream) -> Self {
        let buf_reader = BufReader::new(stream);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .collect();

        let mut headers = HashMap::new();
        let mut body = HashMap::new();
        let mut version = String::new();
        let mut method = HttpMethod::GET;
        if http_request.len() > 0 {
            let first_line: Vec<&str> = http_request[0].split("/").collect();
            version = first_line[1..].join("/");
            method = HttpMethod::from_str(first_line[0].trim());

            for (i, line) in http_request[1..].iter().enumerate() {
                let header: Vec<&str> = line.split(":").collect();
                headers.insert(String::from(header[0]), String::from(&header[1..].concat()));
                if line.starts_with("Content-Length:") {
                    let body_json = &http_request[i + 3..].concat();
                    body = JsonParser::new(body_json).parse_to_map();
                    break;
                }
            }
        }

        HttpRequest {
            version: String::from(version),
            method,
            headers,
            body
        }
    }
}

pub(crate) struct  HttpConnection {
    tcp_stream: TcpStream,
    socket_ddr: SocketAddr,
    request: HttpRequest
}

impl HttpConnection {
    pub(crate) fn new(connection: (TcpStream, SocketAddr)) -> Self {
        HttpConnection {
            request: HttpRequest::new(&connection.0),
            tcp_stream: connection.0,
            socket_ddr: connection.1
        }
    }

    pub(crate) fn get_request(&self) -> &HttpRequest {
        &self.request
    }

    pub(crate) fn response(mut self) {
        // TODO write response
        let status_line = "HTTP/1.1 404 NOT FOUND";
        let contents = "66666";
        let length = contents.len();

        let response = format!(
            "{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}"
        );
        self.tcp_stream.write_all(response.as_bytes()).unwrap();
    }
}

pub(crate) struct HttpServer {
    host: String,
    port: u32,
    pub(crate) listener: TcpListener,
}

impl HttpServer {
    pub(crate) fn bind(host: &str, port: u32) -> Self {
        HttpServer {
            host: String::from(host),
            port,
            listener: TcpListener::bind(format!("{}:{}", host, port)).unwrap(),
        }
    }

    pub(crate) fn receive_connection(&self) -> HttpConnection {
        let connection = self.listener.accept().unwrap();
        return HttpConnection::new(connection);
    }
}