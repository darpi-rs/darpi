use async_trait::async_trait;
use chrono::{Duration, Utc};
use darpi::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION},
    middleware,
    response::ResponderError,
    Body, Request,
};
use derive_more::Display;
pub use jsonwebtoken::*;
use serde::{Deserialize, Serialize};
use shaku::{Component, Interface};
use std::sync::Arc;

pub type Token = String;

#[derive(Debug, Deserialize, Serialize)]
pub struct Claims {
    sub: String,
    role: String,
    exp: usize,
}

impl Claims {
    pub fn role(&self) -> &str {
        &self.role
    }
}

/// authorize provides users the ability to control access to certain or all routes
/// Simply pass it along in the handler macro and provide the #[handler] argument
///  `T: UserRole`
///```rust,ignore
/// #[handler({
///     middleware: {
///         request: [authorize(Role::Admin)]
///     }
/// })]
/// async fn do_something() -> String {
///     format!("do something")
/// }
///```
#[middleware(Request)]
pub async fn authorize(
    #[handler] role: impl UserRole,
    #[request] rp: &Request<Body>,
    #[inject] algo_provider: Arc<dyn JwtAlgorithmProvider>,
    #[inject] token_ext: Arc<dyn TokenExtractor>,
    #[inject] secret_provider: Arc<dyn JwtSecretProvider>,
) -> Result<Claims, Error> {
    let token_res = token_ext.extract(&rp).await;
    match token_res {
        Ok(jwt) => {
            let decoded = decode::<Claims>(
                &jwt,
                secret_provider.decoding_key().await,
                &Validation::new(algo_provider.algorithm().await),
            )
            .map_err(|_| Error::JWTTokenError)?;

            if !role.is_authorized(&decoded.claims) {
                return Err(Error::NoPermissionError);
            }

            Ok(decoded.claims)
        }
        Err(e) => return Err(e),
    }
}

/// UserRole represents user types within an application
/// to identify access levels
///
/// ```rust, ignore
/// #[derive(Clone, PartialEq, PartialOrd)]
/// pub enum Role {
///     User,
///     Admin,
/// }
///
///
///
/// impl Role {
///     pub fn from_str(role: &str) -> Role {
///         match role {
///             "Admin" => Role::Admin,
///             _ => Role::User,
///         }
///     }
/// }
///
/// impl UserRole for Role {
///     fn is_authorized(&self, claims: &Claims) -> bool {
///         let other = Self::from_str(claims.role());
///         &other >= self
///     }
/// }
///
/// impl fmt::Display for Role {
///     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
///         match self {
///             Role::User => write!(f, "User"),
///             Role::Admin => write!(f, "Admin"),
///         }
///     }
/// }
///```
pub trait UserRole: ToString + 'static + Sync + Send {
    fn is_authorized(&self, claims: &Claims) -> bool;
}

/// This type extracts the jwt token from the request header
/// It is a default implementation of `TokenExtractor` and users can choose to
/// implement their own
#[derive(Component)]
#[shaku(interface = TokenExtractor)]
pub struct TokenExtractorImpl;

#[async_trait]
impl TokenExtractor for TokenExtractorImpl {
    async fn extract(&self, r: &Request<Body>) -> Result<Token, Error> {
        jwt_from_header(&r.headers())
    }
}

#[async_trait]
pub trait TokenExtractor: Interface {
    async fn extract(&self, p: &Request<Body>) -> Result<Token, Error>;
}

#[derive(Component)]
#[shaku(interface = JwtSecretProvider)]
pub struct JwtSecretProviderImpl {
    #[shaku(default = unimplemented!())]
    encoding_key: jsonwebtoken::EncodingKey,
    #[shaku(default = unimplemented!())]
    decoding_key: jsonwebtoken::DecodingKey<'static>,
}

#[async_trait]
impl JwtSecretProvider for JwtSecretProviderImpl {
    async fn encoding_key(&self) -> &jsonwebtoken::EncodingKey {
        &self.encoding_key
    }
    async fn decoding_key(&self) -> &jsonwebtoken::DecodingKey<'static> {
        &self.decoding_key
    }
}

#[async_trait]
pub trait JwtSecretProvider: Interface {
    async fn encoding_key(&self) -> &jsonwebtoken::EncodingKey;
    async fn decoding_key(&self) -> &jsonwebtoken::DecodingKey<'static>;
}

#[derive(Component)]
#[shaku(interface = JwtAlgorithmProvider)]
pub struct JwtAlgorithmProviderImpl {
    algorithm: Algorithm,
}

#[async_trait]
impl JwtAlgorithmProvider for JwtAlgorithmProviderImpl {
    async fn algorithm(&self) -> Algorithm {
        self.algorithm
    }
}

#[async_trait]
pub trait JwtAlgorithmProvider: Interface {
    async fn algorithm(&self) -> Algorithm;
}

#[derive(Component)]
#[shaku(interface = JwtTokenCreator)]
pub struct JwtTokenCreatorImpl {
    #[shaku(inject)]
    secret_provider: Arc<dyn JwtSecretProvider>,
    #[shaku(inject)]
    algo_provider: Arc<dyn JwtAlgorithmProvider>,
}

#[async_trait]
impl JwtTokenCreator for JwtTokenCreatorImpl {
    async fn create(
        &self,
        uid: &str,
        role: &dyn UserRole,
        valid_for: Duration,
    ) -> Result<Token, Error> {
        let expiration = Utc::now()
            .checked_add_signed(valid_for)
            .expect("valid timestamp")
            .timestamp();

        let claims = Claims {
            sub: uid.to_owned(),
            role: role.to_string(),
            exp: expiration as usize,
        };
        let header = Header::new(self.algo_provider.algorithm().await);
        encode(&header, &claims, self.secret_provider.encoding_key().await)
            .map_err(|e| Error::JWTTokenCreationError(e))
    }
}

#[async_trait]
pub trait JwtTokenCreator: Interface {
    async fn create(
        &self,
        uid: &str,
        role: &dyn UserRole,
        valid_for: Duration,
    ) -> Result<Token, Error>;
}

const BEARER: &str = "Bearer ";

fn jwt_from_header(headers: &HeaderMap<HeaderValue>) -> Result<String, Error> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => return Err(Error::NoAuthHeaderError),
    };
    let auth_header = match std::str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(Error::NoAuthHeaderError),
    };
    if !auth_header.starts_with(BEARER) {
        return Err(Error::InvalidAuthHeaderError);
    }
    Ok(auth_header.trim_start_matches(BEARER).to_owned())
}

#[derive(Display, Debug)]
pub enum Error {
    #[display(fmt = "wrong credentials")]
    WrongCredentialsError,
    #[display(fmt = "jwt token not valid")]
    JWTTokenError,
    #[display(fmt = "jwt token creation error {}", _0)]
    JWTTokenCreationError(jsonwebtoken::errors::Error),
    #[display(fmt = "no auth header")]
    NoAuthHeaderError,
    #[display(fmt = "invalid auth header")]
    InvalidAuthHeaderError,
    #[display(fmt = "no permission")]
    NoPermissionError,
}

impl ResponderError for Error {}
