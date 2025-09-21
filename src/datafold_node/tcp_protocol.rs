use crate::log_feature;
use crate::logging::features::LogFeature;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::FoldDbResult;

/// Maximum allowed request size in bytes.
const MAX_REQUEST_SIZE: usize = 10_000_000;

/// Read a JSON request from the given TCP stream.
///
/// The function reads a length-prefixed JSON message and deserializes it into
/// a [`serde_json::Value`]. If the peer disconnects while reading, `Ok(None)`
/// is returned.
pub async fn read_request(socket: &mut TcpStream) -> FoldDbResult<Option<Value>> {
    let request_len = match socket.read_u32().await {
        Ok(len) => len as usize,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                log_feature!(LogFeature::TcpServer, info, "Client disconnected");
                return Ok(None);
            }
            log_feature!(
                LogFeature::TcpServer,
                error,
                "Error reading request length: {}",
                e
            );
            return Err(e.into());
        }
    };

    if request_len > MAX_REQUEST_SIZE {
        log_feature!(
            LogFeature::TcpServer,
            warn,
            "Request too large: {} bytes",
            request_len
        );
        let error_response = json!({
            "error": "Request too large",
            "max_size": MAX_REQUEST_SIZE,
        });
        send_response(socket, &error_response).await?;
        return Ok(None);
    }

    let mut request_bytes = vec![0u8; request_len];
    match socket.read_exact(&mut request_bytes).await {
        Ok(_) => {}
        Err(e) => {
            log_feature!(LogFeature::TcpServer, error, "Error reading request: {}", e);
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                log_feature!(
                    LogFeature::TcpServer,
                    info,
                    "Client disconnected while reading request"
                );
                return Ok(None);
            }
            let error_response = json!({
                "error": format!("Error reading request: {}", e),
            });
            let _ = send_response(socket, &error_response).await;
            return Err(e.into());
        }
    };

    match serde_json::from_slice(&request_bytes) {
        Ok(req) => Ok(Some(req)),
        Err(e) => {
            log_feature!(
                LogFeature::TcpServer,
                error,
                "Error deserializing request: {}",
                e
            );
            let error_response = json!({
                "error": format!("Error deserializing request: {}", e),
            });
            let _ = send_response(socket, &error_response).await;
            Ok(None)
        }
    }
}

/// Send a JSON response using the length-prefixed protocol.
pub async fn send_response(socket: &mut TcpStream, value: &Value) -> FoldDbResult<()> {
    let bytes = serde_json::to_vec(value)?;
    socket.write_u32(bytes.len() as u32).await?;
    socket.write_all(&bytes).await?;
    socket.flush().await?;
    Ok(())
}
