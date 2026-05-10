use chrono::{DateTime, Utc};
use jsonwebtoken::{EncodingKey, dangerous::insecure_decode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TokenClaimsError {
    #[error("failed to decode JWT: {0}")]
    Decode(#[from] jsonwebtoken::errors::Error),
    #[error("missing `exp` claim in token")]
    MissingExpiration,
    #[error("invalid `exp` value `{0}`")]
    InvalidExpiration(i64),
    #[error("missing `sub` claim in token")]
    MissingSubject,
    #[error("invalid `sub` value: {0}")]
    InvalidSubject(String),
}

#[derive(Debug, Deserialize)]
struct ExpClaim {
    exp: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct SubClaim {
    sub: Option<String>,
}

/// Extract the expiration timestamp from a JWT without verifying its signature.
pub fn extract_expiration(token: &str) -> Result<DateTime<Utc>, TokenClaimsError> {
    let data = insecure_decode::<ExpClaim>(token)?;
    let exp = data.claims.exp.ok_or(TokenClaimsError::MissingExpiration)?;
    DateTime::from_timestamp(exp, 0).ok_or(TokenClaimsError::InvalidExpiration(exp))
}

/// Extract the subject (user ID) from a JWT without verifying its signature.
pub fn extract_subject(token: &str) -> Result<Uuid, TokenClaimsError> {
    let data = insecure_decode::<SubClaim>(token)?;
    let sub = data.claims.sub.ok_or(TokenClaimsError::MissingSubject)?;
    Uuid::parse_str(&sub).map_err(|_| TokenClaimsError::InvalidSubject(sub))
}

#[derive(Debug, Serialize)]
struct SyntheticClaims {
    sub: String,
    exp: usize,
    iat: usize,
}

/// Create a synthetic JWT for local development mode.
/// The token is NOT cryptographically validated anywhere —
/// HS256 with a static dev secret is sufficient.
pub fn create_synthetic_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp() as usize;
    let claims = SyntheticClaims {
        sub: user_id.to_string(),
        iat: now,
        exp: now + 86400,
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &EncodingKey::from_secret(b"local-dev-no-validation"),
    )
}
