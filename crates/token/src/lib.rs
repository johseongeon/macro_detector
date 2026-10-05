//! 신뢰도 토큰.
//!
//! 형식: `base64url(claims JSON) "." base64url(Ed25519 서명)`
//! 서명은 전송된 claims 바이트 그대로에 대해 검증하므로 JSON 정규화가 필요 없다.
//!
//! 클라이언트 점수는 위조될 수 있으므로, 서버는 `features`로 점수를 다시 계산해
//! 표본 검증한다(Phase 3).

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TOKEN_VERSION: u8 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenClaims {
    pub version: u8,
    pub session_id: String,
    /// 서버가 발급한 일회용 값. 재사용(replay) 방지.
    pub nonce: String,
    pub issued_at_ms: u64,
    /// 0~100, 높을수록 사람.
    pub score: u8,
    /// `guard_scorer::Tier` 값.
    pub tier: u8,
    /// 서버 재검증용 특징 요약 벡터.
    pub features: Vec<f32>,
    /// 실행 중인 브라우저 바이너리 해시(hex).
    pub binary_hash: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TokenError {
    #[error("malformed token")]
    Malformed,
    #[error("invalid signature")]
    BadSignature,
    #[error("unsupported token version {0}")]
    UnsupportedVersion(u8),
}

pub fn encode(claims: &TokenClaims, key: &SigningKey) -> String {
    let body = serde_json::to_vec(claims).expect("claims are always serializable");
    let sig = key.sign(&body);
    format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(&body),
        URL_SAFE_NO_PAD.encode(sig.to_bytes())
    )
}

pub fn decode_and_verify(token: &str, key: &VerifyingKey) -> Result<TokenClaims, TokenError> {
    let (body_b64, sig_b64) = token.split_once('.').ok_or(TokenError::Malformed)?;
    let body = URL_SAFE_NO_PAD
        .decode(body_b64)
        .map_err(|_| TokenError::Malformed)?;
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| TokenError::Malformed)?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| TokenError::Malformed)?;

    key.verify(&body, &sig)
        .map_err(|_| TokenError::BadSignature)?;

    let claims: TokenClaims = serde_json::from_slice(&body).map_err(|_| TokenError::Malformed)?;
    if claims.version != TOKEN_VERSION {
        return Err(TokenError::UnsupportedVersion(claims.version));
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn claims() -> TokenClaims {
        TokenClaims {
            version: TOKEN_VERSION,
            session_id: "s-1".into(),
            nonce: "n-1".into(),
            issued_at_ms: 1_700_000_000_000,
            score: 91,
            tier: 3,
            features: vec![1.0, 2.5],
            binary_hash: "abc".into(),
        }
    }

    #[test]
    fn roundtrips() {
        let k = key();
        let t = encode(&claims(), &k);
        assert_eq!(decode_and_verify(&t, &k.verifying_key()), Ok(claims()));
    }

    #[test]
    fn rejects_tampered_body() {
        let k = key();
        let t = encode(&claims(), &k);
        let mut forged = claims();
        forged.score = 100;
        let forged_body = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&forged).unwrap());
        let sig = t.split_once('.').unwrap().1;
        let forged_token = format!("{forged_body}.{sig}");
        assert_eq!(
            decode_and_verify(&forged_token, &k.verifying_key()),
            Err(TokenError::BadSignature)
        );
    }

    #[test]
    fn rejects_other_key() {
        let t = encode(&claims(), &key());
        let other = SigningKey::from_bytes(&[8u8; 32]).verifying_key();
        assert_eq!(decode_and_verify(&t, &other), Err(TokenError::BadSignature));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(
            decode_and_verify("nope", &key().verifying_key()),
            Err(TokenError::Malformed)
        );
    }
}
