use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::prelude::*;
use std::io::{BufRead, Write};
use std::path::{self, Path, PathBuf};
use std::str::FromStr;
// Uncomment this block to pass the first stage
use anyhow::Result;
use itertools::Itertools;
use std::net::TcpListener;
use std::net::TcpStream;

const NOTFOUND: &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";
const OK: &'static str = "HTTP/1.1 200 Ok\r\n";

#[derive(Debug, Clone)]
struct Request {
    method: String,
    paths: Vec<String>,
    headers: HashMap<String, String>,
}
impl Request {
    fn new(mut reader: std::io::BufReader<&TcpStream>) -> Result<Request> {
        let mut start_line = String::new();
        reader.read_line(&mut start_line)?;
        let mut iter = start_line.split_whitespace();
        if let Some(method) = iter.next() {
            if let Some(path) = iter.next() {
                let paths = path.split("/").skip(1).collect::<Vec<&str>>();

                let mut hdrs: HashMap<String, String> = HashMap::new();
                let mut line = String::new();
                loop {
                    reader.read_line(&mut line)?;
                    if line.is_empty() || line == "\r\n".to_string() {
                        break;
                    }
                    match line.split_once(":") {
                        Some((k, v)) => {
                            hdrs.entry(String::from(k.trim_end_matches(":")))
                                .or_insert(String::from(v.trim()));
                        }
                        _ => (),
                    }

                    line.clear();
                }
                return Ok(Self {
                    method: String::from(method),
                    paths: paths.into_iter().map(|s| s.to_string()).collect(),
                    headers: hdrs,
                });
            }
        }
        Ok(Request {
            method: "UNKNOWN".to_string(), // "GET
            paths: Vec::new(),
            headers: HashMap::new(),
        })
    }
}

#[derive(Debug)]
struct Context {
    root_dir: String,
    stream: TcpStream,
}

impl Context {
    fn new(root_dir: &str, stream: TcpStream) -> Self {
        Self { root_dir: root_dir.to_string(), stream }
    }
}

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
async fn handle_request_get(request: &Request, ctx: &Context) -> Result<()> {
    let mut stream = &ctx.stream;
    match request.paths[0].as_str() {
        "" => {
            stream.write(OK.as_bytes())?;
            stream.write("\r\n".as_bytes())?;
            stream.flush()?;
            return Ok(());
        }
        "echo" => {
            if request.paths.len() < 2 {
                return respond_error(&stream);
            }
            let content = request.paths[1..].join("/");
            return respond_content(&stream, &content);
        }
        "user-agent" => {
            if let Some(user_agent) = request.headers.get("User-Agent") {
                return respond_content(&stream, user_agent);
            } else {
                return respond_error(&stream);
            }
        }
        "files" => {
            println!("paths: {:?}", request.paths);
            println!("root_dir: {:?}", ctx.root_dir);
            let paths: PathBuf = request.paths.iter().skip(1).collect();
            let mut file = PathBuf::new();
            file.push(&ctx.root_dir);
            file.push(&paths);

            if let Ok(mut file) = File::open(file.to_path_buf()) {
                let mut buf = [0; 4096];
                stream.write(OK.as_bytes())?;
                let hdr = format!(
                    "Content-Type: pplication/octet-stream\r\nContent-Length: {}\r\n\r\n",
                    file.metadata().unwrap().len());
                stream.write(hdr.as_bytes())?;

                loop {
                    if let Ok(n) = file.read(&mut buf) {
                        if  n == 0 {
                            stream.flush()?;
                            break;
                        }
                        stream.write(&buf[..n])?;
                    } else {
                        stream.write(NOTFOUND.as_bytes())?;
                        stream.flush()?;
                    }
                }
            } else {
                stream.write(NOTFOUND.as_bytes())?;
                stream.flush()?;
            }
        }

        _ => {}
    }
    Ok(())
}

async fn handle_request(ctx: &Context) -> Result<()> {
    let stream = &ctx.stream;
    let reader = std::io::BufReader::new(stream);
    if let Ok(request) = Request::new(reader) {
        match request.method.as_str() {
            "GET" => {
                return handle_request_get(&request, ctx).await;
            }
            _ => {}
        }

    }

    respond_error(&stream)
}

#[tokio::main]
async fn main() -> Result<()> {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    let args: Vec<String> = std::env::args().collect::<Vec<String>>();
    let root_dir = &args[2];

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");

                let ctx = Context::new(root_dir, stream);
                tokio::spawn(async move {
                    return handle_request(&ctx).await;
                });
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
    Ok(())
}
