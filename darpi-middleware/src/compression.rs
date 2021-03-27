use async_compression::futures::bufread::{BrotliDecoder, DeflateDecoder, GzipDecoder};
use async_compression::futures::write::{BrotliEncoder, DeflateEncoder, GzipEncoder};
use async_trait::async_trait;
use darpi::header::{ToStrError, ACCEPT_ENCODING, CONTENT_ENCODING};
use darpi::hyper::http::HeaderValue;
use darpi::{middleware, response::ResponderError, Body, Request, Response, StatusCode};
use darpi_headers::{AcceptEncoding, ContentEncoding, EncodingType, Error as ContentEncodingError};
use derive_more::Display;
use futures_util::{AsyncReadExt, AsyncWriteExt};
use std::convert::TryFrom;

/// encoding_types is a slice reference provided by the user of the middleware
/// the order of the elements establishes their priority
/// the priority of the middleware user will be matched against the client's Accept-Encoding header
/// the match with the highest client weight will be the chosen compression algorithm
/// if non is found, it will result in a noop
/// in this example, we can all requests to all handlers will be compressed with gzip
///  if the client supports it
///```rust
/// #[tokio::test]
/// async fn main() -> Result<(), darpi::Error> {
///     let address = format!("127.0.0.1:{}", 3000);
///     app!({
///         address: address,
///         module: make_container => Container,
///         // a set of global middleware that will be executed for every handler
///         middleware: [compress(&[Gzip])],
///         bind: [
///             {
///                 route: "/login",
///                 method: Method::POST,
///                 // the POST method allows this handler to have
///                 // Json<Name> as an argument
///                 handler: login
///             },
///         ],
///     })
///    .run()
///    .await
/// }
/// ```
#[middleware(Response)]
pub async fn compress(
    #[handler] encoding_types: &'static [EncodingType],
    #[response] r: &mut Response<Body>,
) -> Result<(), Error> {
    let matched_encoding = if let Some(val) = r.headers().get(&ACCEPT_ENCODING) {
        if let Ok(enc) = val.to_str() {
            let mut matched_types = vec![];

            for et in encoding_types {
                matched_types.push(AcceptEncoding::parse(enc, *et));
            }
            matched_types.sort();
            matched_types
                .first()
                .map_or(EncodingType::Identity, |f| f.encoding)
        } else {
            EncodingType::Identity
        }
    } else {
        EncodingType::Identity
    };

    let mut b = r.body_mut();
    let mut full_body = darpi::body::to_bytes(&mut b)
        .await
        .map_err(|e| Error::ReadBody(e))?;

    match matched_encoding {
        EncodingType::Gzip => {
            full_body = Gzip.encode(&full_body).await?.into();
        }
        EncodingType::Deflate => {
            full_body = Deflate.encode(&full_body).await?.into();
        }
        EncodingType::Br => {
            full_body = Brotli.encode(&full_body).await?.into();
        }
        _ => {}
    };

    *b = Body::from(full_body);

    let new_header = HeaderValue::from_str(matched_encoding.into()).expect("not to happen");

    if let Some(hv) = r.headers_mut().get_mut(&CONTENT_ENCODING) {
        *hv = new_header;
    } else {
        r.headers_mut().insert(CONTENT_ENCODING, new_header);
    }

    Ok(())
}

/// this middleware will decompress multiple compressions
/// it supports [Gzip, Deflate, Br, Identity, Auto] and any other compression
/// will result in an error response with StatusCode::UNSUPPORTED_MEDIA_TYPE
/// in this example, all requests to all handlers will be decompressed before
/// the handler gets invoked
///```rust
/// #[tokio::test]
/// async fn main() -> Result<(), darpi::Error> {
///     let address = format!("127.0.0.1:{}", 3000);
///     app!({
///         address: address,
///         module: make_container => Container,
///         // a set of global middleware that will be executed for every handler
///         middleware: [decompress()],
///         bind: [
///             {
///                 route: "/login",
///                 method: Method::POST,
///                 // the POST method allows this handler to have
///                 // Json<Name> as an argument
///                 handler: login
///             },
///         ],
///     })
///    .run()
///    .await
/// }
/// ```
#[middleware(Request)]
pub async fn decompress(#[request] r: &mut Request<Body>) -> Result<(), Error> {
    let mut full_body = darpi::body::to_bytes(r.body_mut())
        .await
        .map_err(|e| Error::ReadBody(e))?;

    if let Some(ce) = r.headers().get(&CONTENT_ENCODING) {
        let encodings =
            ContentEncoding::try_from(&*ce).map_err(|e| Error::InvalidContentEncoding(e))?;
        for et in encodings.into_iter() {
            match et {
                EncodingType::Gzip => {
                    full_body = Gzip.decode(&full_body).await?.into();
                }
                EncodingType::Deflate => {
                    full_body = Deflate.decode(&full_body).await?.into();
                }
                EncodingType::Br => {
                    full_body = Brotli.decode(&full_body).await?.into();
                }
                EncodingType::Identity | EncodingType::Auto => {}
            }
        }
    }

    *r.body_mut() = Body::from(full_body);
    Ok(())
}

pub struct Brotli;

#[async_trait]
impl Encoder for Brotli {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Br
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = BrotliEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Brotli {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = BrotliDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

pub struct Deflate;

#[async_trait]
impl Encoder for Deflate {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Deflate
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = DeflateEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Deflate {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = DeflateDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

pub struct Gzip;

#[async_trait]
impl Encoder for Gzip {
    fn encoding_type(&self) -> EncodingType {
        EncodingType::Gzip
    }
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let x: Vec<u8> = vec![];
        let mut writer = GzipEncoder::new(x);

        writer
            .write_all(bytes)
            .await
            .map_err(|e| Error::EncodingIOError(e))?;
        Ok(writer.into_inner().into())
    }
}

#[async_trait]
impl Decoder for Gzip {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let mut g = GzipDecoder::new(bytes);
        let mut x: Vec<u8> = vec![];
        g.read_to_end(&mut x)
            .await
            .map_err(|e| Error::DecodingIOError(e))?;
        Ok(x)
    }
}

#[async_trait]
pub trait Encoder {
    fn encoding_type(&self) -> EncodingType;
    async fn encode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error>;
}

#[async_trait]
pub trait Decoder {
    async fn decode(&self, bytes: &[u8]) -> Result<Vec<u8>, Error>;
}

#[derive(Display, Debug)]
pub enum Error {
    #[display(fmt = "encoding error {}", _0)]
    EncodingIOError(std::io::Error),
    #[display(fmt = "decoding error {}", _0)]
    DecodingIOError(std::io::Error),
    #[display(fmt = "read body error {}", _0)]
    ReadBody(darpi::hyper::Error),
    #[display(fmt = "invalid content encoding error {}", _0)]
    InvalidContentEncoding(ContentEncodingError),
    ToStrError(ToStrError),
    UnsupportedContentEncoding(EncodingType),
}

impl ResponderError for Error {
    fn status_code(&self) -> StatusCode {
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    }
}
