use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::net::{TcpListener};
use std::ops::Index;
use std::string::ToString;
use std::vec;
use regex::Regex;
use crate::http::base::{HttpConnection, HttpMethod, HttpContext, HttpResponse, HttpStatus};

struct EndPoint{
    url: String,
    method: HttpMethod,
    pub func: Box<dyn Fn(HttpContext) -> HttpResponse>
}

impl EndPoint {
    fn new(url: &str, method: HttpMethod, func: Box<dyn Fn(HttpContext) -> HttpResponse>) -> Self {
        EndPoint{
            url: url.to_string(),
            method,
            func
        }
    }
}

impl PartialEq<Self> for EndPoint {
    fn eq(&self, other: &Self) -> bool {
        other.url == self.url && other.method == self.method
    }
}

impl Eq for EndPoint {}

impl Hash for EndPoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.url.hash(state);
        self.method.hash(state)
    }
}

pub(crate) struct HttpServer {
    host: String,
    port: u32,
    listener: Option<TcpListener>,
    dispatcher: RequestDispatcher
}

impl HttpServer {
    pub(crate) fn bind(host: &str, port: u32) -> Self {
        HttpServer {
            host: String::from(host),
            port,
            listener: None,
            dispatcher: RequestDispatcher::new(),
        }
    }

    pub(crate) fn register_end_point(&mut self,
                                     url: &str,
                                     method: HttpMethod,
                                     func: Box<dyn Fn(HttpContext) -> HttpResponse>) {
        let mut dispatcher = &mut self.dispatcher;
        dispatcher.register_end_point(url, method, func);
    }

    pub(crate) fn start(&mut self) {
        match self.listener {
            None => { self.listener = Some(TcpListener::bind(format!("{}:{}", self.host, self.port)).unwrap()) }
            Some(_) => {}
        }

        let listener = self.listener.as_ref().unwrap();

        loop {
            let accepted = listener.accept().unwrap();
            let connection = HttpConnection::new(accepted);
            self.dispatcher.dispatch(connection)
        }
    }
}

#[derive(Debug)]
struct PathParamParser{
    path_param: Vec<String>,
    url_path_pattern_regex: Regex,
    pattern_str: String,
}

impl PartialEq for PathParamParser {
    fn eq(&self, other: &Self) -> bool {
        self.pattern_str == other.pattern_str
    }
}
impl  PathParamParser  {
    fn new(path_param: Vec<String>, url: &str) -> PathParamParser {
        let regex = Regex::new(r"\{([\w-]+)}").unwrap();
        let mut pattern_str = regex.replace_all(url, "([\\w-]+)").to_string();
        pattern_str.push('$');
        let url_path_pattern_regex = Regex::new(pattern_str.as_str()).unwrap();
        PathParamParser {
            path_param,
            url_path_pattern_regex,
            pattern_str
        }
    }

    fn is_match(&self, url: &str) -> bool {
        self.url_path_pattern_regex.is_match(url)
    }

    pub(crate) fn parse(&self, url: &str) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for cap in self.url_path_pattern_regex.captures_iter(url) {
            for (i, string) in self.path_param.iter().enumerate() {
                map.insert(string.clone(), cap[i + 1].to_string());
            }
        }
        map
    }

}


struct RequestDispatcher {
    endpoints_pure_url: HashMap<String, HashSet<EndPoint>>,
    endpoints_path_param_url: Vec<(PathParamParser, HashSet<EndPoint>)>,
    path_param_pattern:  Regex
}

impl RequestDispatcher {
    fn new() -> Self {
        RequestDispatcher {
            endpoints_pure_url: HashMap::new(),
            endpoints_path_param_url: vec![],
            path_param_pattern:  Regex::new(r"\{([\w-]+)}").unwrap()
        }
    }
    fn register_end_point(&mut self,
                                 url: &str,
                                 method: HttpMethod,
                                 func: Box<dyn Fn(HttpContext) -> HttpResponse>) {
        match url.split_once("?") {
            Some((_, _)) => {
                panic!("`{}` has query parameters, they are not allowed when defining the endpoint!", url)
            },
            _ => {}
        }
        let mut inserted = false;
        if self.path_param_pattern.is_match(url) {
            let path_params: Vec<String> = self.path_param_pattern.captures_iter(url)
                                                                    .map(|x| x[1].to_string())
                                                                    .collect();
            let parser = PathParamParser::new(path_params, url);
            let exist = self.endpoints_path_param_url.iter_mut()
                                                        .filter(|(p,hs)| *p == parser)
                                                        .take(1)
                                                        .next();

            if let Some((_, endpoints)) = exist {
                endpoints.insert(EndPoint::new(url, method, func));
            } else {
                let mut set = HashSet::new();
                set.insert(EndPoint::new(url, method, func));
                self.endpoints_path_param_url.push((parser, set));
            }
        } else {
            inserted = self.endpoints_pure_url.entry(url.to_string())
                                            .or_insert(HashSet::new())
                                            .insert(EndPoint::new(url, method, func));
        }

        if inserted {
            panic!("`{:?} {}` is already used by another endpoint", method, url)
        }
    }

    fn find_possible_endpoints_pure_url(&self, url: &str) -> Option<&HashSet<EndPoint>> {
        match self.endpoints_pure_url.get(&url.to_string()) {
            None => {None}
            Some(endpoints) => {Some(endpoints)}
        }
    }
    fn find_possible_endpoints_path_url(&self, url: &str) -> Option<(HashMap<String, String>, &HashSet<EndPoint>)> {
        self.endpoints_path_param_url.iter()
            .filter(|x| x.0.is_match(url))
            .map(|x| (x.0.parse(url), &(x.1)))
            .next()
    }

    fn dispatch(&mut self, connection: HttpConnection) {
        let request = &connection.request;
        let endpoints_pure_url = match self.find_possible_endpoints_pure_url(&request.path){
            None => {None}
            Some(endpoints) => {
                endpoints.iter()
                                .filter(|e| e.method == request.method)
                                .take(1)
                                .next()
            }
        };

        let response = match endpoints_pure_url {
            None => {
                match self.find_possible_endpoints_path_url(&request.path) {
                    None => {HttpResponse::build_response(HttpStatus::NOT_FOUND, None)}
                    Some(endpoints) => {
                        let endpoint = endpoints.1.iter()
                            .filter(|e| e.method == request.method).take(1).next();
                        if endpoint.is_none() {
                            HttpResponse::build_response(HttpStatus::NOT_ALLOWED, None)
                        }else{
                            let endpoint: &EndPoint = endpoint.unwrap();
                            let func = &(*endpoint.func);
                            func(HttpContext::new(endpoints.0, request))
                        }
                    }
                }
            }
            Some(endpoint) => {
                let func = &(*endpoint.func);
                func(HttpContext::new(HashMap::new(), request))
            }
        };

        connection.response(response)
    }
}