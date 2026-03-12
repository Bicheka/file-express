use clap::{Parser, Subcommand};
use anyhow::Ok;
use p2ps::{CertificateDer, PrivateKeyDer};
use tokio::net::TcpListener;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::fs::File;
use std::fs;
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

        #[arg(short)]
        expected_client_hash: String
    },

    /// Send file to receiver
    Send {
        #[arg(short, long)]
        path: String,

        #[arg(short, long)]
        to: String, // format: IP:PORT

        #[arg(short)]
        expected_server_hash: String
    },
    Generate,
    Hash
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Listen { port, path, expected_client_hash } => {
            listen(port, &path, expected_client_hash).await?;
        }
        Commands::Send { path, to, expected_server_hash } => {
            send(&path, &to, expected_server_hash).await?;
        }
        Commands::Generate {} => {
            generate()?;
        },
        Commands::Hash {} => {
            get_hash()?;
        }
    }

    Ok(())
}

use crate::compression::unzip_file;

pub async fn listen(port: u16, download_path: &str, expected_client_hash: String) -> Result<()> {

    let (server_cert, server_key) = p2ps::generate_identity()?;


    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("Listening on {}", port);


    let addr = listener.local_addr()?;

    println!("Listening on {}:{}", addr, port);

    // let (mut socket, _) = listener.accept().await?;
    let mut secure_conn = p2ps::accept(&listener, server_cert, server_key, expected_client_hash).await?;

    // Read directory flag
    let is_dir = secure_conn.stream.read_u8().await? == 1;

    // Read filename
    let name_len = secure_conn.stream.read_u64().await?;
    let mut name_buf = vec![0u8; name_len as usize];
    secure_conn.stream.read_exact(&mut name_buf).await?;
    let filename = String::from_utf8(name_buf)?;

    // Read file size
    let file_size = secure_conn.stream.read_u64().await?;

    // Create full download path
    let mut full_path = PathBuf::from(download_path);
    full_path.push(&filename);

    // Ensure parent directories exist
    if let Some(parent) = full_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Receive file
    let mut file = File::create(&full_path).await?;
    let mut received = 0;
    let mut buffer = [0u8; 64 * 1024];

    while received < file_size {
        let n = secure_conn.stream.read(&mut buffer).await?;
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

pub async fn send(path: &str, addr: &str, expected_server_hash: String) -> Result<()> {
    let identity_path = stringify!(get_identity_dir());
    let client_cert_bytes = get_identity_file("identity.cert").expect(&format!("Could not find identity cert in path: {}, try running ***file-express generate*** first", identity_path));
    let client_cert = CertificateDer::try_from(client_cert_bytes)?;

    let client_key_bytes = get_identity_file("identity.key").expect(&format!("Could not find identity key in path: {}, try running ***file-express generate*** first", identity_path));
    let client_key = PrivateKeyDer::try_from(client_key_bytes).unwrap();
    let mut client_conn = p2ps::connect(addr, expected_server_hash, client_cert, client_key).await?;
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

    // Send directory flag
    client_conn.stream.write_u8(if is_dir { 1 } else { 0 }).await?;

    // Send filename
    client_conn.stream.write_u64(filename.len() as u64).await?;
    client_conn.stream.write_all(filename.as_bytes()).await?;

    // Send file size
    client_conn.stream.write_u64(metadata.len()).await?;

    // Stream file
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).await?;
        if n == 0 { break; }
        client_conn.stream.write_all(&buffer[..n]).await?;
    }

    if is_dir {
        tokio::fs::remove_file(&actual_path).await?;
    }

    println!("Transfer complete.");
    Ok(())
}

fn generate() -> anyhow::Result<()> {
    let (cert, key) = p2ps::generate_identity()?;

    // get user home directory
    let identity_dir = get_identity_dir();
    println!("path: {:?}", identity_dir);
    // store certs and keys there
    fs::create_dir_all(&identity_dir)?;
    fs::write(identity_dir.join("identity.cert"), cert)?;
    fs::write(identity_dir.join("identity.key"), key.secret_der())?;
    get_hash().expect("Could not generate hash");
    Ok(())
}

fn get_hash() -> anyhow::Result<String>{
    // get cert
    let cert_bytes = get_identity_file("identity.cert")?;
    let cert = CertificateDer::try_from(cert_bytes)?;
    let hash = p2ps::get_cert_fingerprint(&cert);
    println!("{}", hash);
    return Ok(hash);
}

fn get_identity_file(file_name: &str) -> anyhow::Result<Vec<u8>>{
    let cert_path = get_identity_dir()
    .join(file_name);
    Ok(fs::read(cert_path)?)
}

fn get_identity_dir() -> PathBuf{
    let home_dir= std::env::home_dir().expect("Impossible to get your home dir!");
    let dir: PathBuf = home_dir
    .join(".file_express");
    dir
}