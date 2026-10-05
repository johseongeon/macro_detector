use std::net::SocketAddr;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::VerifyingKey;
use guard_verify_api::{router, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // TODO(Phase 3): 단일 키 대신 디바이스 등록 시 발급한 키 레지스트리에서 조회.
    let key_b64 = std::env::var("GUARD_VERIFYING_KEY")
        .map_err(|_| "GUARD_VERIFYING_KEY (base64url Ed25519 public key) is required")?;
    let key_bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(key_b64.trim())?
        .try_into()
        .map_err(|_| "GUARD_VERIFYING_KEY must be 32 bytes")?;
    let verifying_key = VerifyingKey::from_bytes(&key_bytes)?;

    let addr: SocketAddr = std::env::var("GUARD_LISTEN_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".into())
        .parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "verify-api listening");

    axum::serve(listener, router(AppState::new(verifying_key))).await?;
    Ok(())
}
