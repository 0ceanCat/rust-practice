use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::str::FromStr;
use crate::utils::json::{DataType, JsonParser};

pub(crate) struct MediaType;

impl<'a> MediaType {
    const APPLICATION_XML: &'a str = "application/xml";
    const APPLICATION_ATOM_XML: &'a str = "application/atom+xml";
    const APPLICATION_XHTML_XML: &'a str = "application/xhtml+xml";
    const APPLICATION_SVG_XML: &'a str = "application/svg+xml";
    const APPLICATION_JSON: &'a str = "application/json";
    const APPLICATION_FORM_URLENCODED: &'a str = "application/x-www-form-urlencoded";
    const MULTIPART_FORM_DATA: &'a str = "multipart/form-data";
    const APPLICATION_OCTET_STREAM: &'a str = "application/octet-stream";
    const TEXT_PLAIN: &'a str = "text/plain";
    const TEXT_XML: &'a str = "text/xml";
    const TEXT_HTML: &'a str = "text/html";
    const SERVER_SENT_EVENTS: &'a str = "text/event-stream";
    const APPLICATION_JSON_PATCH_JSON: &'a str = "application/json-patch+json";
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
            return query_params
        }
        return HashMap::new()
    }
}

struct HttpContext {
    path_params: HashMap<String, String>,
    request: HttpRequest,
}

pub(crate) struct HttpResponse {
    status: u32,
    data: Option<String>,
}

impl HttpResponse {
    pub(crate) fn ok() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            data: None,
        }
    }

    pub(crate) fn ok_with_data(data: String) -> HttpResponse {
        HttpResponse {
            status: HttpStatus::OK,
            data: Some(data),
        }
    }

    pub(crate) fn bad_request() -> HttpResponse {
        HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            data: None,
        }
    }

    pub(crate) fn bad_request_with_data(data: String) -> HttpResponse {
        HttpResponse {
            status: HttpStatus::BAD_REQUEST,
            data: Some(data),
        }
    }

    pub(crate) fn build_response(status: u32, data: Option<String>) -> HttpResponse {
        HttpResponse {
            status,
            data,
        }
    }
}

#[derive(Debug)]
pub(crate) struct HttpConnection {
    tcp_stream: TcpStream,
    socket_ddr: SocketAddr,
    pub(crate) request: HttpRequest,
}

impl<'a> HttpConnection {
    const DEFAULT_MEDIA_TYPE: &'a str = MediaType::TEXT_PLAIN;

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

        let default = HttpConnection::DEFAULT_MEDIA_TYPE.to_string();
        let media_type = self.request.headers.get("Accept").unwrap_or(&default);

        let response = format!("{status_line}\r\nContent-type: {media_type}\r\nContent-Length: {length}\r\n\r\n{contents}");
        self.tcp_stream.write_all(response.as_bytes()).unwrap();
    }
}