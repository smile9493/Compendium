//! JWT authentication and authorization scaffolding.
//!
//! Provides:
//! - Token generation with claims (subject, expiry, roles)
//! - Token validation and claims extraction
//! - Axum `FromRequest` guard for protected routes
//! - Tower middleware for automatic token verification
//!
//! # Feature flag
//!
//! Enable with `features = ["auth"]` in `Cargo.toml`.
//!
//! # Usage
//!
//! ```ignore
//! use pdf_common::auth::{AuthLayer, Claims, generate_token, validate_token};
//!
//! let token = generate_token("user-1", &["admin"], &secret, 24)?;
//! let claims = validate_token(&token, &secret)?;
//! ```

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub roles: Vec<String>,
}

impl Claims {
    pub fn new(subject: impl Into<String>, roles: &[impl AsRef<str>], ttl_hours: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as usize;

        Self {
            sub: subject.into(),
            exp: now + (ttl_hours as usize * 3600),
            iat: now,
            roles: roles.iter().map(|r| r.as_ref().to_string()).collect(),
        }
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|r| self.has_role(r))
    }
}

/// Generate a signed JWT token.
pub fn generate_token(
    subject: impl Into<String>,
    roles: &[impl AsRef<str>],
    secret: &str,
    ttl_hours: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::new(subject, roles, ttl_hours);
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Validate a JWT token and extract claims.
pub fn validate_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Refresh a token by issuing a new one with the same claims but updated expiry.
pub fn refresh_token(
    token: &str,
    secret: &str,
    ttl_hours: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = validate_token(token, secret)?;
    let new_claims = Claims::new(&claims.sub, &claims.roles, ttl_hours);
    encode(
        &Header::default(),
        &new_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

// ─── Axum integration ────────────────────────────────────────

#[cfg(feature = "auth-axum")]
pub mod axum_integration {
    use super::*;
    use axum::extract::FromRequestParts;
    use axum::http::request::Parts;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use axum::{async_trait, Json};
    use std::sync::Arc;
    use tower::{Layer, Service};
    use std::task::{Context, Poll};

    /// Shared JWT secret and optional required roles.
    #[derive(Clone)]
    pub struct AuthConfig {
        pub secret: Arc<String>,
        pub required_roles: Vec<String>,
    }

    impl AuthConfig {
        pub fn new(secret: impl Into<String>) -> Self {
            Self {
                secret: Arc::new(secret.into()),
                required_roles: vec![],
            }
        }

        pub fn with_roles(mut self, roles: Vec<String>) -> Self {
            self.required_roles = roles;
            self
        }
    }

    /// Extract authenticated user claims from request headers.
    ///
    /// Usage as an axum extractor:
    /// ```ignore
    /// async fn protected_route(auth: AuthenticatedUser) -> impl IntoResponse {
    ///     format!("Hello, {}!", auth.claims.sub)
    /// }
    /// ```
    #[derive(Debug, Clone)]
    pub struct AuthenticatedUser {
        pub claims: Claims,
        pub token: String,
    }

    #[async_trait]
    impl<S> FromRequestParts<S> for AuthenticatedUser
    where
        S: Send + Sync,
    {
        type Rejection = Response;

        async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
            let auth_config = parts
                .extensions
                .get::<AuthConfig>()
                .cloned()
                .ok_or_else(|| {
                    (StatusCode::INTERNAL_SERVER_ERROR, "AuthConfig not found").into_response()
                })?;

            let header = parts
                .headers
                .get("Authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .ok_or_else(|| {
                    (
                        StatusCode::UNAUTHORIZED,
                        Json(serde_json::json!({"error": "Missing or invalid Authorization header"})),
                    )
                        .into_response()
                })?;

            let claims = validate_token(header, &auth_config.secret).map_err(|e| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error": format!("Invalid token: {}", e)})),
                )
                    .into_response()
            })?;

            if !auth_config.required_roles.is_empty()
                && !claims.has_any_role(&auth_config.required_roles.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            {
                return Err((
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Insufficient permissions"})),
                )
                    .into_response());
            }

            Ok(AuthenticatedUser {
                claims,
                token: header.to_string(),
            })
        }
    }

    /// Tower Layer that inserts AuthConfig into request extensions.
    #[derive(Clone)]
    pub struct AuthLayer {
        config: AuthConfig,
    }

    impl AuthLayer {
        pub fn new(secret: impl Into<String>) -> Self {
            Self {
                config: AuthConfig::new(secret),
            }
        }

        pub fn with_roles(secret: impl Into<String>, roles: Vec<String>) -> Self {
            Self {
                config: AuthConfig::new(secret).with_roles(roles),
            }
        }
    }

    impl<S> Layer<S> for AuthLayer {
        type Service = AuthMiddleware<S>;

        fn layer(&self, inner: S) -> Self::Service {
            AuthMiddleware {
                inner,
                config: self.config.clone(),
            }
        }
    }

    #[derive(Clone)]
    pub struct AuthMiddleware<S> {
        inner: S,
        config: AuthConfig,
    }

    impl<S, B> Service<axum::http::Request<B>> for AuthMiddleware<S>
    where
        S: Service<axum::http::Request<B>> + Clone + Send + 'static,
        S::Future: Send + 'static,
        B: Send + 'static,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, mut req: axum::http::Request<B>) -> Self::Future {
            req.extensions_mut().insert(self.config.clone());
            self.inner.call(req)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

    #[test]
    fn generate_and_validate_token() {
        let token = generate_token("user-1", &["admin", "reader"], TEST_SECRET, 1).unwrap();
        let claims = validate_token(&token, TEST_SECRET).unwrap();
        assert_eq!(claims.sub, "user-1");
        assert!(claims.has_role("admin"));
        assert!(claims.has_role("reader"));
        assert!(!claims.has_role("writer"));
    }

    #[test]
    fn has_any_role() {
        let claims = Claims::new("user-2", &["reader"], 1);
        assert!(claims.has_any_role(&["admin", "reader"]));
        assert!(!claims.has_any_role(&["admin", "writer"]));
    }

    #[test]
    fn expired_token_fails() {
        let claims = Claims {
            sub: "user-3".into(),
            exp: 1,
            iat: 1,
            roles: vec![],
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(TEST_SECRET.as_bytes()),
        )
        .unwrap();
        let result = validate_token(&token, TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn invalid_secret_fails() {
        let token = generate_token("user-4", &[] as &[&str], TEST_SECRET, 1).unwrap();
        let result = validate_token(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn refresh_token_works() {
        let token = generate_token("user-5", &["user"], TEST_SECRET, 1).unwrap();
        let refreshed = refresh_token(&token, TEST_SECRET, 24).unwrap();
        let claims = validate_token(&refreshed, TEST_SECRET).unwrap();
        assert_eq!(claims.sub, "user-5");
    }
}