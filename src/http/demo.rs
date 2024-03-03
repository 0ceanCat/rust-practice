use std::fs::File;
use std::io::Read;
use std::net::SocketAddr;
use crate::http::base::{HttpConnection, HttpContext, HttpMethod, HttpResponse, HttpStatus};
use crate::http::http_core::HttpServer;

fn main() {
    let mut server = HttpServer::bind("127.0.0.1", 7878);
    server.register_end_point("/abc/{username}/{id}", HttpMethod::GET, Box::new(test));
    server.register_end_point("/images/{image-id}", HttpMethod::GET, Box::new(get_image));
    server.do_before(Box::new(filter)); // executed before starting process the request
    server.do_after(Box::new(do_after)); // executed after the request has been processed
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
    response.set_header(String::from("Server Name"), String::from("yoo"));
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

fn get_image(r: HttpContext) -> HttpResponse {
    let image_id = r.get_path_param("image-id").unwrap();
    let file_path = format!(r"images\{}.jpg", image_id);
    let mut file = File::open(file_path).unwrap();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    return HttpResponse::build_response(HttpStatus::OK, Some(buffer));
}