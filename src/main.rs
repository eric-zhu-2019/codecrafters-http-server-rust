use std::io::{BufRead, Write};
// Uncomment this block to pass the first stage
use anyhow::Result;
use std::net::TcpListener;
use std::net::TcpStream;

const NOTFOUND: &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";
const OK: &'static str = "HTTP/1.1 200 Ok\r\n";

fn respond_error(mut stream: &TcpStream) -> Result<()> {
    stream.write(NOTFOUND.as_bytes())?;
    stream.flush()?;
    Ok(())
}
fn respond_content(mut stream: &TcpStream, content: &str) -> Result<()> {
    stream.write(OK.as_bytes())?;
    let hdr = format!(
        "Content-Type: text/plain\r\nContent-Length: {}\r\n\r\n",
        content.len()
    );
    stream.write(hdr.as_bytes())?;
    stream.write(content.as_bytes())?;
    stream.flush()?;
    Ok(())
}
async fn handle_request(mut stream: TcpStream) -> Result<()> {
    let reader = std::io::BufReader::new(&stream);
    let mut line_iter = reader.lines();
    let start_line = line_iter.next().unwrap()?;

    let mut iter = start_line.split_whitespace();
    if let Some("GET") = iter.next() {
        let path = iter.next().unwrap();
        let paths = path.split("/").skip(1).collect::<Vec<&str>>();

        match paths[0] {
            "" => {
                stream.write(OK.as_bytes())?;
                stream.write("\r\n".as_bytes())?;
                stream.flush()?;
                return Ok(());
            }
            "echo" => {
                if paths.len() < 2 {
                    return respond_error(&stream);
                }
                let content = paths[1..].join("/");
                return respond_content(&stream, &content);
            }
            "user-agent" => {
                //line_iter.for_each(|line| println!("{}", line.as_ref().unwrap()));

                if let Some(user_agent) =
                    line_iter.find(|line| line.as_ref().unwrap().starts_with("User-Agent:"))
                {
                    return respond_content(
                        &stream,
                        user_agent?.split_whitespace().collect::<Vec<&str>>()[1],
                    );
                } else {
                    return respond_error(&stream);
                }
            }

            _ => {}
        }
    }
    respond_error(&stream)
}

#[tokio::main]
async fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                tokio::spawn(async move {
                    return handle_request(stream).await;
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
