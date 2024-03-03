use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;
use crate::utils::json::{DataType, JsonParser};

pub(crate) struct MediaType;

impl<'a> MediaType {
    pub(crate) const APPLICATION_XML: &'a str = "application/xml";
    pub(crate) const APPLICATION_ATOM_XML: &'a str = "application/atom+xml";
    pub(crate) const APPLICATION_XHTML_XML: &'a str = "application/xhtml+xml";
    pub(crate) const APPLICATION_SVG_XML: &'a str = "application/svg+xml";
    pub(crate) const APPLICATION_JSON: &'a str = "application/json";
    pub(crate) const APPLICATION_FORM_URLENCODED: &'a str = "application/x-www-form-urlencoded";
    pub(crate) const MULTIPART_FORM_DATA: &'a str = "multipart/form-data";
    pub(crate) const APPLICATION_OCTET_STREAM: &'a str = "application/octet-stream";
    pub(crate) const TEXT_PLAIN: &'a str = "text/plain";
    pub(crate) const TEXT_XML: &'a str = "text/xml";
    pub(crate) const TEXT_HTML: &'a str = "text/html";
    pub(crate) const IMAGE_JPEG: &'a str = "image/jpeg";
    pub(crate) const IMAGE_PNG: &'a str = "image/png";
    pub(crate) const SERVER_SENT_EVENTS: &'a str = "text/event-stream";
    pub(crate) const APPLICATION_JSON_PATCH_JSON: &'a str = "application/json-patch+json";
}

pub(crate) struct HttpHeader;
impl<'a> HttpHeader {
    pub(crate) const CONTENT_TYPE: &'a str = "Content-type";
    pub(crate) const CONTENT_LENGTH: &'a str = "Content-length";
    pub(crate) const ACCEPT: &'a str = "Accept";
    pub(crate) const CONTENT: &'a str = "Content";
    pub(crate) const USER_AGENT: &'a str = "User-Agent";
}

#[derive(Debug, Default, Hash, Copy, Clone, PartialEq, Eq)]
pub(crate) enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
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
pub(crate) struct HttpStatus;

impl HttpStatus {
    pub(crate) const OK: u32 = 200;
    pub(crate) const BAD_REQUEST: u32 = 400;
    pub(crate) const FORBIDDEN: u32 = 401;
    pub(crate) const NOT_FOUND: u32 = 404;
    pub(crate) const NOT_ALLOWED: u32 = 405;
    pub(crate) const INTERNAL_ERROR: u32 = 500;
}

#[derive(Debug)]
pub(crate) struct HttpRequest {
    pub(crate) version: String,
    pub(crate) path: String,
    pub(crate) method: HttpMethod,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) query_params: HashMap<String, String>,
    pub(crate) body: HashMap<String, DataType>,
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

        let query_params: HashMap<String, String> = Self::parse_query_params(path);
        let headers: HashMap<String, String> = Self::parse_header(header);

        let body = Self::parse_body(&mut reader, &headers)?;

        Some(HttpRequest {
            method,
            path: path.to_string(),
            query_params: query_params,
            version: version.trim().to_string(),
            headers,
            body,
        })
    }

    fn parse_body(reader: &mut BufReader<&TcpStream>, headers: &HashMap<String, String>) -> Option<HashMap<String, DataType>> {
        let body = match headers.get(HttpHeader::CONTENT_LENGTH) {
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
        Some(body)
    }

    fn parse_header(header_str: &str) -> HashMap<String, String> {
        header_str
            .split("\r\n")
            .filter_map(|x| x.split_once(':'))
            .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
            .collect()
    }
    fn parse_query_params(url_path: &str) -> HashMap<String, String> {
        if let Some(tuple) = url_path.split_once("?") {
            let query_params = tuple.1.split("&")
                .filter_map(|query| query.split_once("="))
                .map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
                .collect();
            return query_params;
        }
        return HashMap::new();
    }
}

pub(crate) struct HttpContext<'a> {
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub request: &'a HttpRequest,
}

impl<'a> HttpContext<'a> {
    pub fn new(path_params: HashMap<String, String>, query_params: HashMap<String, String>, request: &'a HttpRequest) -> Self {
        HttpContext {
            path_params,
            query_params,
            request,
        }
    }

    pub fn get_path_param(&self, path_variable: &str) -> Option<&String> {
        self.path_params.get(path_variable)
    }

    pub fn get_query_param(&self, query_variable: &str) -> Option<&String> {
        self.query_params.get(query_variable)
    }
}

pub(crate) struct HttpResponse {
    pub(crate) status: u32,
    pub(crate) headers: HashMap<String, String>,
    pub(crate) data: Option<Vec<u8>>,
}

impl HttpResponse {

    pub(crate) fn set_header(&mut self, key: String, value:String) {
        self.headers.insert(key, value);
    }

    pub(crate) fn ok() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            headers: HashMap::new(),
            data: None,
        }
    }

    pub(crate) fn ok_with_data(data: Vec<u8>) -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            headers: HashMap::new(),
            data: Some(data),
        }
    }

    pub(crate) fn bad_request() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            headers: HashMap::new(),
            data: None,
        }
    }

    pub(crate) fn bad_request_with_data(data: Vec<u8>) -> HttpResponse {
        HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            headers: HashMap::new(),
            data: Some(data),
        }
    }

    pub(crate) fn build_response(status: u32, data: Option<Vec<u8>>) -> HttpResponse {
        let mut headers = HashMap::new();
        HttpResponse {
            status,
            headers,
            data
        }
    }
}

#[derive(Debug)]
pub(crate) struct HttpConnection {
    tcp_stream: TcpStream,
    pub(crate) socket_addr: SocketAddr,
    pub(crate) request: HttpRequest,
}

impl<'a> HttpConnection {
    const DEFAULT_MEDIA_TYPE: &'a str = MediaType::TEXT_PLAIN;
    const BREAK_LINE: &'a str = "\r\n";

    pub(crate) fn new(connection: (TcpStream, SocketAddr)) -> Self {
        HttpConnection {
            request: HttpRequest::new(&connection.0).unwrap(),
            tcp_stream: connection.0,
            socket_addr: connection.1,
        }
    }

    pub(crate) fn response(mut self, response: HttpResponse) {
        let response_bytes = Self::build_response_string(self.request, response);
        self.tcp_stream.write_all(&response_bytes).unwrap();
    }

    fn build_response_string(request: HttpRequest, http_response: HttpResponse) -> Vec<u8> {
        let status_line = format!("{} {} OK", request.version, http_response.status.to_string());
        let mut response_detail = String::new();
        let mut headers = http_response.headers.clone();

        response_detail.push_str(status_line.as_str());
        response_detail.push_str(Self::BREAK_LINE);
        headers.iter().for_each(|(k, v)| {
            response_detail.push_str(k.as_str());
            response_detail.push_str(": ");
            response_detail.push_str(v.as_str());
            response_detail.push_str(Self::BREAK_LINE);
        });

        let content = http_response.data.unwrap_or(vec![]);
        response_detail.push_str("Content-Length: ");
        response_detail.push_str(content.len().to_string().as_str());
        response_detail.push_str(Self::BREAK_LINE);
        response_detail.push_str(Self::BREAK_LINE);
        let mut response_detail = response_detail.into_bytes();
        response_detail.extend(content);
        response_detail
    }
}