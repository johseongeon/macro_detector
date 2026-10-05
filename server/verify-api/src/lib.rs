//! Fast path 토큰 검증 API. 모델 추론 없이 서명·신선도만 확인한다 (목표 p99 < 5ms).

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ed25519_dalek::VerifyingKey;
use guard_scorer::Tier;
use guard_token::{decode_and_verify, TokenError};
use serde::{Deserialize, Serialize};

/// 토큰 유효 시간. 점수는 클라이언트에서 계속 갱신되므로 짧게 둔다.
const MAX_TOKEN_AGE_MS: u64 = 30_000;
/// 클라이언트·서버 시계 차이 허용치.
const MAX_CLOCK_SKEW_MS: u64 = 5_000;

pub struct AppState {
    verifying_key: VerifyingKey,
}

impl AppState {
    pub fn new(verifying_key: VerifyingKey) -> Arc<Self> {
        Arc::new(Self { verifying_key })
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/v1/verify", post(verify))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct VerifyRequest {
    pub token: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Allow,
    Challenge,
    Block,
}

#[derive(Serialize, Deserialize)]
pub struct VerifyResponse {
    pub decision: Decision,
    pub score: u8,
}

pub enum ApiError {
    InvalidToken(TokenError),
    Expired,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let msg = match self {
            ApiError::InvalidToken(e) => e.to_string(),
            ApiError::Expired => "token expired".to_string(),
        };
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response()
    }
}

async fn verify(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let claims =
        decode_and_verify(&req.token, &state.verifying_key).map_err(ApiError::InvalidToken)?;

    let now = now_ms();
    if claims.issued_at_ms > now + MAX_CLOCK_SKEW_MS
        || now.saturating_sub(claims.issued_at_ms) > MAX_TOKEN_AGE_MS
    {
        return Err(ApiError::Expired);
    }
    // TODO(Phase 3): nonce 일회성 검사(Redis), features 기반 점수 재계산 표본 검증,
    //                Observe 단계는 Deep Analyzer에 비동기 분석 요청.

    let decision = match Tier::from_u8(claims.tier) {
        Some(Tier::Trusted | Tier::Observe) => Decision::Allow,
        Some(Tier::Block) => Decision::Block,
        // 알 수 없는 값은 차단하지 않고 확인 단계로 보낸다(오판 최소화).
        Some(Tier::Challenge) | None => Decision::Challenge,
    };
    Ok(Json(VerifyResponse {
        decision,
        score: claims.score,
    }))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use ed25519_dalek::SigningKey;
    use guard_token::{encode, TokenClaims, TOKEN_VERSION};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(&[1u8; 32])
    }

    fn token(tier: Tier, issued_at_ms: u64) -> String {
        let claims = TokenClaims {
            version: TOKEN_VERSION,
            session_id: "s".into(),
            nonce: "n".into(),
            issued_at_ms,
            score: 90,
            tier: tier as u8,
            features: vec![],
            binary_hash: "h".into(),
        };
        encode(&claims, &signing_key())
    }

    async fn call(token: String) -> (StatusCode, serde_json::Value) {
        let app = router(AppState::new(signing_key().verifying_key()));
        let req = Request::post("/v1/verify")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({ "token": token }).to_string(),
            ))
            .unwrap();
        let res = app.oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn allows_trusted() {
        let (status, body) = call(token(Tier::Trusted, now_ms())).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["decision"], "allow");
    }

    #[tokio::test]
    async fn blocks_block_tier() {
        let (_, body) = call(token(Tier::Block, now_ms())).await;
        assert_eq!(body["decision"], "block");
    }

    #[tokio::test]
    async fn rejects_expired() {
        let (status, _) = call(token(Tier::Trusted, now_ms() - MAX_TOKEN_AGE_MS - 1)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rejects_bad_signature() {
        let other = SigningKey::from_bytes(&[2u8; 32]);
        let claims = guard_token::decode_and_verify(
            &token(Tier::Trusted, now_ms()),
            &signing_key().verifying_key(),
        )
        .unwrap();
        let (status, _) = call(encode(&claims, &other)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
}
