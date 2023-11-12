use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, Write};
use std::path::PathBuf;
// Uncomment this block to pass the first stage
use anyhow::{Result, anyhow};
use std::net::TcpListener;
use std::net::TcpStream;

const NOTFOUND: &'static str = "HTTP/1.1 404 Not Found\r\n\r\n";
const OK: &'static str = "HTTP/1.1 200 Ok\r\n";
const OK201: &'static str = "HTTP/1.1 201 Created\r\n";

#[derive(Debug, Clone)]
struct Request {
    method: String,
    paths: Vec<String>,
    headers: HashMap<String, String>,
}
impl Request {
    fn new(reader: &mut BufReader<&TcpStream>) -> Result<Request> {
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
                    println!("line: {:?}", line);
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
        Self {
            root_dir: root_dir.to_string(),
            stream,
        }
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
                let mut buf = [0; 1024 * 1024];
                stream.write(OK.as_bytes())?;
                let hdr = format!(
                    "Content-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n",
                    file.metadata().unwrap().len()
                );
                stream.write(hdr.as_bytes())?;

                loop {
                    if let Ok(n) = file.read(&mut buf) {
                        if n == 0 {
                            stream.flush()?;
                            break;
                        }
                        stream.write(&buf[..n])?;
                    } else {
                        stream.flush()?;
                        break;
                    }
                }
            } else {
                stream.write(NOTFOUND.as_bytes())?;
                stream.flush()?;
            }
        }

        _ => {
            stream.write(NOTFOUND.as_bytes())?;
            stream.flush()?;
        }
    }
    Ok(())
}

fn upload_file(request: &Request, filepath: PathBuf, reader: &mut BufReader<&TcpStream>) -> Result<()> {
    if let Ok(mut file) = File::create(filepath.to_path_buf()) {
        // get the length of file
        println!("request: {:?}", request);
        if let Some(lenstr) = request.headers.get("Content-Length") {
            if let Ok(mut len) = usize::from_str_radix(lenstr, 10) {
                println!("len={}", len);
                loop {
                    let mut buf = [0; 4096];
                    match reader.read(&mut buf) {
                        Ok(0)  => {
                            break;
                        }
                        Ok(n) => {
                            println!("n={}", n);
                            len -= n;
                            file.write_all(&mut buf)?;
                            if len == 0 {
                                break;
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            println!("would block");
                            break;
                        }
                        Err(e) => {
                            return Err(anyhow!("read error: {}", e));
                        }
                    }
                }
            }
        } else {
            return Err(anyhow!("no content-length"));
        }
    } else {
        return Err(anyhow!("create file failed"));
    }
    println!("upload file {:?} ok", filepath.to_str());

    Ok(())
}

async fn handle_request_post(request: &Request, ctx: &Context, reader: &mut BufReader<&TcpStream>) -> Result<()> {
    let mut stream = &ctx.stream;
    match request.paths[0].as_str() {
        "files" => {
            // create file firslty
            let paths: PathBuf = request.paths.iter().skip(1).collect();
            let mut file = PathBuf::from(&ctx.root_dir);
            file.push(&paths);
            match upload_file(&request, file, reader) {
                Ok(()) => {
                    stream.write(OK201.as_bytes())?;
                    stream.flush()?;
                }
                e => {
                    stream.write(NOTFOUND.as_bytes())?;
                    stream.flush()?;
                    return e;
                }
            }
        }
        _ => {
            let _ = respond_error(stream);
        }
    }

    Ok(())
}

async fn handle_request(ctx: &Context) -> Result<()> {
    let stream = &ctx.stream;
    let mut reader = BufReader::new(stream);
    if let Ok(request) = Request::new(&mut reader) {
        match request.method.as_str() {
            "GET" => {
                return handle_request_get(&request, ctx).await;
            }
            "POST" => {
                return handle_request_post(&request, ctx, &mut reader).await;
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
    let mut root_dir = String::new();
    if args.len() >= 3 {
        root_dir = args[2].to_string();
    }

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");

                let ctx = Context::new(root_dir.as_str(), stream);
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
