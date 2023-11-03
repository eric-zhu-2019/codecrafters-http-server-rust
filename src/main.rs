use std::io::Write;
// Uncomment this block to pass the first stage
use std::net::TcpListener;
use std::net::TcpStream;
use anyhow::Result;
use tokio::io::AsyncBufReadExt;

async fn handle_request(mut stream: TcpStream) -> Result<()> {
    let stream1 = tokio::net::TcpStream::from_std(stream.try_clone()?)?;
    let reader = tokio::io::BufReader::new(stream1);
    let mut lines = reader.lines();
    if let Some(start_line) = lines.next_line().await? {
        let mut iter = start_line.split_whitespace();
        if let Some("GET") = iter.next()  {
             if let Some(path) = iter.next() {
                 if path == "/" {
                     let ok = b"HTTP/1.1 200 OK\r\n\r\n";
                     stream.write(ok)?;
                 } else {
                    let not_found = b"HTTP/1.1 404 NOT FOUND\r\n\r\n";
                    stream.write(not_found)?;
                 }
                 stream.flush()?;
             }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()>{
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_request(stream).await?;
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
