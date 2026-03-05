use clap::{Parser, Subcommand};
use anyhow::Ok;
use local_ip_address::local_ip;
use tokio::net::TcpListener;

use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use std::net::IpAddr;
use std::path::PathBuf;
use anyhow::Result;

use crate::compression::start_compressing;
mod compression;
#[derive(Parser)]
#[command(name = "fexpress")]
#[command(about = "Simple file transfer CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start receiver and wait for files
    Listen {
        #[arg(long)]
        port: u16,

        #[arg(long)]
        path: String,
    },

    /// Send file to receiver
    Send {
        #[arg(short, long)]
        path: String,

        #[arg(short, long)]
        to: String, // format: IP:PORT
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Listen { port, path } => {
            listen(port, &path).await?;
        }
        Commands::Send { path, to } => {
            send(&path, &to).await?;
        }
    }

    Ok(())
}

use crate::compression::unzip_file;

pub fn get_local_ip() -> anyhow::Result<IpAddr> {
    Ok(local_ip()?)
}

pub async fn listen(port: u16, download_path: &str) -> Result<()> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Listening on {}", port);


    let ip = local_ip()?;

    println!("Listening on {}:{}", ip, port);

    let (mut socket, _) = listener.accept().await?;

    // 1️⃣ Read directory flag
    let is_dir = socket.read_u8().await? == 1;

    // 2️⃣ Read filename
    let name_len = socket.read_u64().await?;
    let mut name_buf = vec![0u8; name_len as usize];
    socket.read_exact(&mut name_buf).await?;
    let filename = String::from_utf8(name_buf)?;

    // 3️⃣ Read file size
    let file_size = socket.read_u64().await?;

    // 4️⃣ Create full download path
    let mut full_path = PathBuf::from(download_path);
    full_path.push(&filename);

    // Ensure parent directories exist
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 5️⃣ Receive file
    let mut file = File::create(&full_path).await?;
    let mut received = 0;
    let mut buffer = [0u8; 64 * 1024];

    while received < file_size {
        let n = socket.read(&mut buffer).await?;
        if n == 0 { break; }
        file.write_all(&buffer[..n]).await?;
        received += n as u64;
    }

    println!("Received: {}", full_path.display());

    if is_dir {
        println!("Unzipping...");
        unzip_file(full_path.to_str().unwrap(), full_path.with_extension("").to_str().unwrap())?;
        tokio::fs::remove_file(&full_path).await?;
        println!("Directory extracted.");
    }

    Ok(())
}

pub async fn send(path: &str, addr: &str) -> Result<()> {
    let mut stream = TcpStream::connect(addr).await?;
    let path = PathBuf::from(path);

    let is_dir = path.is_dir();

    let actual_path = if is_dir {
        let zip_path = path.with_extension("zip");
        tokio::task::spawn_blocking({
            let path = path.clone();
            let zip_path = zip_path.clone();
            move || {
                start_compressing(&path, &zip_path, zip::CompressionMethod::Stored)
                    .expect("Compression failed");
            }
        }).await?;
        zip_path
    } else {
        path.clone()
    };

    let mut file = File::open(&actual_path).await?;
    let metadata = file.metadata().await?;
    let filename = actual_path.file_name().unwrap().to_str().unwrap();

    // 1️⃣ Send directory flag
    stream.write_u8(if is_dir { 1 } else { 0 }).await?;

    // 2️⃣ Send filename
    stream.write_u64(filename.len() as u64).await?;
    stream.write_all(filename.as_bytes()).await?;

    // 3️⃣ Send file size
    stream.write_u64(metadata.len()).await?;

    // 4️⃣ Stream file
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        stream.write_all(&buffer[..n]).await?;
    }

    if is_dir {
        tokio::fs::remove_file(&actual_path).await?;
    }

    println!("Transfer complete.");
    Ok(())
}
