use std::{
    io::{prelude::*, BufReader},
    net::{TcpListener, TcpStream},
};
use std::io::BufWriter;

pub(crate) fn start_server(port: u32) {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        println!("received a connection");
        handle_connection(stream);
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&mut stream);

    let mut writer = BufWriter::new(&mut stream);
    let response = "HTTP/1.1 200 OK\r\n\r\nnb";
    let result = writer.write(response.as_bytes());
    match result {
        Ok(_) => {println!("responded")}
        Err(_) => {println!("error")}
    }
}