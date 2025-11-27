use std::sync::Arc;

use crate::{AppState, Result, error::Error};
use argon2::{
    Argon2, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use argon2::{PasswordHash, PasswordHasher};
use axum::{
    extract::{Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use passwords::PasswordGenerator;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i32,
    pub exp: usize,
    /// issued at
    pub iat: usize,
}

pub fn generate_jwt_token(
    user_id: &i32,
    expires_in: chrono::Duration,
    secret: String,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(expires_in)
        .unwrap()
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_owned(),
        exp: expiration,
        iat: chrono::Utc::now().timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

pub fn verify_jwt_token(
    token: &str,
    secret: String,
) -> Result<Claims, jsonwebtoken::errors::Error> {
    let validation = Validation::default();

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &validation,
    )?;
    Ok(token_data.claims)
}

fn extract_and_verify_jwt(request: &Request, jwt_secret: String) -> Result<Option<Claims>, Error> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    match auth_header {
        Some(header) => {
            let token = header
                .strip_prefix("Bearer ")
                .ok_or(Error::msg("missing auth token").status_code(StatusCode::UNAUTHORIZED))?;

            let claims = verify_jwt_token(token, jwt_secret)
                .map_err(|_| Error::msg("invalid token").status_code(StatusCode::UNAUTHORIZED))?;

            Ok(Some(claims))
        }
        None => Ok(None),
    }
}

fn create_guest_claims() -> Claims {
    let now = chrono::Utc::now().timestamp() as usize;
    Claims {
        sub: -1,
        exp: now + 3600,
        iat: now,
    }
}

pub async fn jwt_auth_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let claims = extract_and_verify_jwt(&request, state.config.jwt_secret.clone())?
        .ok_or(Error::msg("missing auth header").status_code(StatusCode::UNAUTHORIZED))?;

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

pub async fn jwt_auth_accept_guest_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response> {
    let claims = match extract_and_verify_jwt(&request, state.config.jwt_secret.clone())? {
        Some(claims) => claims,
        None => create_guest_claims(),
    };

    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

pub fn hash_password(password: String) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)?
        .to_string();
    Ok(password_hash)
}

pub fn verify_password(password: String, password_hash: String) -> Result<()> {
    let argon2 = Argon2::default();
    let parsed_hash = PasswordHash::new(&password_hash)?;

    argon2
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| Error::msg("incorrect credentials").status_code(StatusCode::UNAUTHORIZED))
}

pub fn generate_strong_password() -> String {
    let pg = PasswordGenerator {
        length: 24,
        numbers: true,
        lowercase_letters: true,
        uppercase_letters: true,
        symbols: false,
        spaces: false,
        exclude_similar_characters: true,
        strict: true,
    };

    pg.generate_one().unwrap()
}
