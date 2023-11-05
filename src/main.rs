use std::io::{BufRead, Write};
// Uncomment this block to pass the first stage
use anyhow::Result;
use std::net::TcpListener;
use std::net::TcpStream;

const NOTFOUND: &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";
const OK: &'static str = "HTTP/1.1 200 Ok\r\n\r\n";

fn respond_error(mut stream: &TcpStream) {
    stream.write(NOTFOUND.as_bytes()).unwrap();
    stream.flush().unwrap();
}
fn handle_request(mut stream: TcpStream) -> Result<()> {
    let mut reader = std::io::BufReader::new(&stream);
    let mut start_line = String::new();
    reader.read_line(&mut start_line)?;

    let mut iter = start_line.split_whitespace();
    if let Some("GET") = iter.next() {
        let path = iter.next().unwrap();
        let paths = path.split("/").skip(1).collect::<Vec<&str>>();

        match paths[0] {
            "" => {
                stream.write(OK.as_bytes())?;
                stream.flush()?;
                return Ok(());
            }
            "echo" => {
                //println!("{:?}", paths);
                if paths.len() < 2 {
                    respond_error(&stream);
                    return Ok(());
                }
                stream.write(OK.as_bytes())?;
                let hdr = format!(
                    "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
                    paths[1].len()
                );
                stream.write(hdr.as_bytes())?;
                stream.write(paths[1].as_bytes())?;
                stream.flush()?;
                return Ok(());
            }
            _ => {}
        }
    }
    stream.write(NOTFOUND.as_bytes())?;
    stream.flush()?;

    Ok(())
}

fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_request(stream)?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
