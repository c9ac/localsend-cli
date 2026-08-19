use smol::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use std::{io, num::ParseIntError};
use thiserror_lite::err_enum;
use zfish::{ProgressBar, ProgressStyle};

err_enum! {
    #[derive(Debug)]
    pub enum RequestError {
        #[error("Statuscode is: {0}")]
        StatusCode(usize),
        #[error("Receiver rejected this operation")]
        Rejection,
        #[error("Illegal response")]
        Response,
        #[error("{0}")]
        ParseIntError(#[from] ParseIntError),
        #[error("{0}")]
        Io(#[from] io::Error),
    }
}

enum TransferType {
    Chunked,
    ContentLength(usize),
}

pub async fn post(base_url: &str, path: &str, content: &[u8]) -> Result<Vec<u8>, RequestError> {
    // Connect to the receive device
    let mut stream = TcpStream::connect(base_url).await?;

    write_head(path, content.len() as u64, &mut stream).await?;
    stream.write_all(content).await?;

    // Get response
    read_http_body(stream).await
}

pub async fn upload_file(base_url: &str, path: &str, file: &mut File) -> Result<(), RequestError> {
    // Connect to the receive device
    let mut stream = TcpStream::connect(base_url).await?;

    // Get file size
    let size = file.metadata().await?.len();

    write_head(path, size, &mut stream).await?;

    // Progress bar
    let mut pb = ProgressBar::new(size).with_style(ProgressStyle::Arrow);

    // Write file content
    let mut buf = [0u8; 1024 * 8];
    loop {
        let n = file.read(&mut buf).await?;

        if n == 0 {
            break;
        }

        stream.write_all(&buf[..n]).await?;
        pb.inc(n as u64);
    }

    Ok(())
}

async fn write_head(
    path: &str,
    content_len: u64,
    stream: &mut TcpStream,
) -> Result<(), RequestError> {
    // build request
    let header = format!(
        "POST {} HTTP/1.1\r\n\
            Content-Length: {}\r\n\
            \r\n",
        path, content_len
    )
    .into_bytes();

    Ok(stream.write_all(&header).await?)
}

async fn read_http_body(stream: TcpStream) -> Result<Vec<u8>, RequestError> {
    // Setup reader
    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    // Get status code
    reader.read_line(&mut line).await?;
    let status_code = line
        .split_whitespace()
        .nth(1)
        .ok_or(RequestError::Response)?
        .parse()?;

    // Judge whether error occurs by status code
    match status_code {
        403 => return Err(RequestError::Rejection),
        c if c >= 300 => return Err(RequestError::StatusCode(c)),
        _ => {}
    };

    // Handle both possibilities
    let transfer_type = judge_transfer_type(&mut reader).await?;

    let body = match transfer_type {
        TransferType::Chunked => read_chunked_body(&mut reader).await?,
        TransferType::ContentLength(len) => {
            let mut buf = vec![0u8; len];
            if len > 0 {
                reader.read_exact(&mut buf).await?;
            }
            buf
        }
    };

    Ok(body)
}

async fn judge_transfer_type(
    reader: &mut BufReader<TcpStream>,
) -> Result<TransferType, RequestError> {
    let mut line = String::new();
    let mut is_chunked = false;
    let mut content_length = None;

    loop {
        line.clear();
        reader.read_line(&mut line).await?;

        // Header ends with "\r\n"
        if line == "\r\n" {
            break;
        }

        let lower = line.to_lowercase();
        if lower.contains("transfer-encoding") && lower.contains("chunked") {
            is_chunked = true;
        } else if line.starts_with("Content-Length") {
            content_length = Some(
                line.split(":")
                    .nth(1)
                    .ok_or(RequestError::Response)?
                    .trim()
                    .parse::<usize>()?,
            );
        }
    }

    // chunked transfer has higher priority
    if is_chunked {
        return Ok(TransferType::Chunked);
    } else if let Some(len) = content_length {
        return Ok(TransferType::ContentLength(len));
    }

    Err(RequestError::Response)
}

async fn read_chunked_body(reader: &mut BufReader<TcpStream>) -> Result<Vec<u8>, RequestError> {
    let mut body = Vec::new();
    let mut line = String::new();

    loop {
        // Get chunk size
        line.clear();
        reader.read_line(&mut line).await?;

        // Handle potential ";"
        let chunk_size = if line.contains(";") {
            line.split(";").next().unwrap_or_default()
        } else {
            &line
        }
        .trim();
        let chunk_size = usize::from_str_radix(chunk_size, 16)?;

        if chunk_size == 0 {
            break;
        }

        // Read content
        let old_len = body.len();
        body.resize(old_len + chunk_size, 0);
        reader.read_exact(&mut body[old_len..]).await?;

        // Skip "\r\n"
        line.clear();
        reader.read_line(&mut line).await?;
    }

    Ok(body)
}
