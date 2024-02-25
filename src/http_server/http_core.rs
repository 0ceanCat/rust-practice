use std::any::Any;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::net::TcpStream;

pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH
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
            _ => {panic!("Unknown method detected")}
        }
    }
}

struct HttpRequest {
    method: HttpMethod,
    headers: HashMap<String, String>,
    body: HashMap<String, Box<dyn Any>>,
}

impl HttpRequest {
    fn new(mut stream: TcpStream) {
        let buf_reader = BufReader::new(&mut stream);
        let http_request_string: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();
        let method = HttpMethod::from_str(&http_request_string[0]);

    }
}