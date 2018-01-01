#[macro_use]
extern crate log;

extern crate http;
extern crate mime;
extern crate futures;
extern crate tokio_core;
extern crate hyper;
extern crate backtrace;
extern crate regex;
extern crate bytes;
#[macro_use]
extern crate lazy_static;

pub mod resource;
pub mod flow;
pub mod server;
pub mod conneg;
pub mod body;
pub mod chunk;
pub mod error;

pub type Body = hyper::Body;

impl body::Body for hyper::Body {
    type Chunk = hyper::Chunk;
    type Error = hyper::Error;

    fn empty() -> Self {
        hyper::Body::empty()
    }

    fn pair() -> (futures::sync::mpsc::Sender<Result<Self::Chunk, <Self as body::Body>::Error>>, Self) {
        hyper::Body::pair()
    }
}

impl From<error::Error> for hyper::Error {
    fn from(e: error::Error) -> hyper::Error {
        hyper::Error::Version
    }
}

impl From<chunk::Chunk> for hyper::Chunk {
    fn from(c: chunk::Chunk) -> hyper::Chunk {
        let bytes: bytes::Bytes = c.into();
        hyper::Chunk::from(bytes)
    }
}