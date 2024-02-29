use std::collections::HashMap;
use std::f32::consts::E;
use std::hash::Hash;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::str::FromStr;
use std::string::ToString;
use crate::utils::json::{DataType, JsonParser};

macro_rules! unwrap_or_default {
    ($option:expr) => {
        match $option {
            Some(value) => value,
            None => return Default::default(),
        }
    };
}

pub(crate) struct MediaType(&'static str);

impl MediaType {
    const APPLICATION_XML: Self = Self("application/xml");
    const APPLICATION_ATOM_XML: Self = Self("application/atom+xml");
    const APPLICATION_XHTML_XML: Self = Self("application/xhtml+xml");
    const APPLICATION_SVG_XML: Self = Self("application/svg+xml");
    const APPLICATION_JSON: Self = Self("application/json");
    const APPLICATION_FORM_URLENCODED: Self = Self("application/x-www-form-urlencoded");
    const MULTIPART_FORM_DATA: Self = Self("multipart/form-data");
    const APPLICATION_OCTET_STREAM: Self = Self("application/octet-stream");
    const TEXT_PLAIN: Self = Self("text/plain");
    const TEXT_XML: Self = Self("text/xml");
    const TEXT_HTML: Self = Self("text/html");
    const SERVER_SENT_EVENTS: Self = Self("text/event-stream");
    const APPLICATION_JSON_PATCH_JSON: Self = Self("application/json-patch+json");
}

#[derive(Debug, Default)]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}


impl PartialEq<Self> for HttpMethod {
    fn eq(&self, other: &Self) -> bool {
        other == self
    }
}

impl FromStr for HttpMethod {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lowercase = s.trim().to_lowercase();
        match lowercase.as_str() {
            "get" => Ok(HttpMethod::GET),
            "post" => Ok(HttpMethod::POST),
            "put" => Ok(HttpMethod::PUT),
            "delete" => Ok(HttpMethod::DELETE),
            "patch" => Ok(HttpMethod::PATCH),
            _ => {
                Err("Unknown method detected".to_string())
            }
        }
    }
}

#[derive(Debug)]
struct  HttpStatus;

impl HttpStatus {
    const OK: u32 = 200;
    const BAD_REQUEST: u32 = 400;
    const FORBIDDEN: u32 = 401;
    const NOT_FOUND: u32 = 404;
    const INTERNAL_ERROR: u32 = 500;
}

#[derive(Debug)]
pub(crate) struct HttpRequest {
    version: String,
    path: String,
    method: HttpMethod,
    headers: HashMap<String, String>,
    query_params: HashMap<String, String>,
    body: HashMap<String, DataType>,
}

impl HttpRequest {
    fn new(stream: &TcpStream) -> Option<Self> {
        let mut reader = BufReader::new(stream);
        let mut buffer = String::new();

        loop {
            reader.read_line(&mut buffer).ok()?;
            if buffer.ends_with("\r\n\r\n") {
                break;
            }
        }

        let (first_line, header) = buffer.split_once('\n')?;


        let first_line: Vec<&str> = first_line.split(" ").collect();
        let method: HttpMethod = first_line[0].trim().parse().ok()?;
        let path = first_line[1].trim();
        let version = first_line[2].trim();

        let headers: HashMap<String, String> = header
            .split("\r\n")
            .filter_map(|x| x.split_once(':'))
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .collect();

        let body = match headers.get("Content-Length") {
            Some(content_length) => {
                let size: usize = content_length.parse().ok()?;
                let mut buffer = vec![0u8; size];
                reader.read_exact(&mut buffer).ok()?;
                buffer
            }
            None => {
                vec![]
            }
        };

        let body = std::str::from_utf8(&body).unwrap();

        let body = JsonParser::new(body).parse_to_map();

        Some(HttpRequest {
            method,
            path: path.to_string(),
            query_params: HashMap::new(),
            version: version.trim().to_string(),
            headers,
            body
        })
    }
}

struct HttpContext {
    path_params: HashMap<String, String>,
    request: HttpRequest
}

pub(crate) struct HttpResponse {
    status: u32,
    data: Option<String>
}

impl HttpResponse {
    pub(crate) fn ok() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            data: None
        }
    }

    pub(crate) fn ok_with_data(data: String) -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            data: Some(data)
        }
    }

    pub(crate) fn bad_request() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            data: None
        }
    }

    pub(crate) fn bad_request_with_data(data: String) -> HttpResponse {
        HttpResponse {
            status: 400,
            data: Some(data)
        }
    }
}

#[derive(Debug)]
pub(crate) struct HttpConnection {
    tcp_stream: TcpStream,
    socket_ddr: SocketAddr,
    request: HttpRequest,
}

impl HttpConnection {
    const DEFAULT_MEDIA_TYPE: MediaType = MediaType::TEXT_PLAIN;

    pub(crate) fn new(connection: (TcpStream, SocketAddr)) -> Self {
        HttpConnection {
            request: HttpRequest::new(&connection.0).unwrap(),
            tcp_stream: connection.0,
            socket_ddr: connection.1,
        }
    }

    pub(crate) fn get_request(&self) -> &HttpRequest {
        &self.request
    }

    pub(crate) fn response(mut self, response: HttpResponse) {
        let status_line = format!("{} {}", self.request.version, response.status.to_string());
        let contents = response.data.unwrap_or(String::new());
        let length = contents.len();

        let default = HttpConnection::DEFAULT_MEDIA_TYPE.0.to_string();
        let media_type = self.request.headers.get("Accept").unwrap_or(&default);

        let response = format!("{status_line}\r\nContent-type: {media_type}\r\nContent-Length: {length}\r\n\r\n{contents}");
        self.tcp_stream.write_all(response.as_bytes()).unwrap();
    }
}

struct EndPoint(String, HttpMethod);

impl EndPoint {
    fn new(url: &str, method: HttpMethod) -> Self {
        EndPoint(url.to_string(), method)
    }
}

impl PartialEq<Self> for EndPoint {
    fn eq(&self, other: &Self) -> bool {
        other.1 == self.1 && other.0 == self.0
    }
}

pub(crate) struct HttpServer {
    host: String,
    port: u32,
    listener: TcpListener,
    endpoints: HashMap<EndPoint, Box<dyn FnMut(HttpRequest) -> HttpResponse>>
}

impl HttpServer {
    pub(crate) fn bind(host: &str, port: u32) -> Self {
        HttpServer {
            host: String::from(host),
            port,
            listener: TcpListener::bind(format!("{}:{}", host, port)).unwrap(),
            endpoints: HashMap::new()
        }
    }

    pub(crate) fn register_end_point(&mut self,
                                     url: &str,
                                     method: HttpMethod,
                                     func: Box<dyn FnMut(HttpRequest) -> HttpResponse>) {
        let mut map = &self.endpoints;

    }

    pub(crate) fn receive_connection(&self) -> HttpConnection {
        let connection = self.listener.accept().unwrap();
        HttpConnection::new(connection)
    }
}
