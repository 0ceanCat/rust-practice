use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::net::{TcpListener};
use std::string::ToString;
use crate::http::base::{HttpConnection, HttpMethod, HttpRequest, HttpResponse, HttpStatus};

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

impl Eq for EndPoint {}

impl Hash for EndPoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
        self.1.hash(state)
    }
}

pub(crate) struct HttpServer {
    host: String,
    port: u32,
    listener: Option<TcpListener>,
    endpoints: HashMap<EndPoint, Box<dyn Fn(&HttpRequest) -> HttpResponse>>,
}

impl HttpServer {
    pub(crate) fn bind(host: &str, port: u32) -> Self {
        HttpServer {
            host: String::from(host),
            port,
            listener: None,
            endpoints: HashMap::new(),
        }
    }

    pub(crate) fn register_end_point(&mut self,
                                     url: &str,
                                     method: HttpMethod,
                                     func: Box<dyn Fn(&HttpRequest) -> HttpResponse>) {
        let mut map = &mut self.endpoints;
        map.insert(EndPoint::new(url, method), func);
    }

    pub(crate) fn start(&mut self) {
        match self.listener {
            None => { self.listener = Some(TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap()) }
            Some(_) => {}
        }

        let listener = self.listener.as_ref().unwrap();
        let map = &mut self.endpoints;
        loop {
            let accepted = listener.accept().unwrap();
            let connection = HttpConnection::new(accepted);
            let request = &connection.request;
            let response = match map.get_mut(&EndPoint::new(&request.path.as_str(), request.method)) {
                None => {
                    HttpResponse::build_response(HttpStatus::NOT_FOUND, None)
                }
                Some(func) => {
                    let func = func.as_ref();
                    let http_response = (*func)(request);
                    http_response
                }
            };
            connection.response(response)
        }
    }
}
