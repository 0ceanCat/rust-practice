use std::net::SocketAddr;
use crate::http::base::{HttpConnection, HttpContext, HttpHeader, HttpMethod, HttpResponse, MediaType};
use crate::http::http_core::HttpServer;

fn main() {
    let mut server = HttpServer::bind("127.0.0.1", 7878);
    server.register_end_point("/abc/{username}/{id}", HttpMethod::GET, Box::new(crate::test));
    server.do_before(Box::new(crate::filter));
    server.do_after(Box::new(crate::do_after));
    server.start()
}

fn filter(c:&HttpConnection) -> bool {
    match c.socket_addr {
        SocketAddr::V4(addr) => {
            addr.ip().to_string() != "127.0.0.1"
        }
        SocketAddr::V6(addr) => {
            true
        }
    }
}

fn do_after(response: &mut HttpResponse) {
    response.set_header(String::from(HttpHeader::CONTENT_TYPE), MediaType::TEXT_PLAIN.to_string());
}

fn test(r: HttpContext) -> HttpResponse {
    let request = r.request;
    println!("path params: {:?}", r.path_params);
    println!("query params: {:?}", r.query_params);
    println!("method: {:?}", request.method);
    println!("version: {:?}", request.version);
    println!("headers: {:?}", request.headers);
    println!("body: {:?}", request.body);
    return HttpResponse::ok_with_data(String::from("nb").into_bytes())
}