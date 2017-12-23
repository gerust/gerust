use chunk::Chunk;
use futures::Stream;
use futures::sync::mpsc;

// TODO: maybe FROM is not the right abstraction here
pub trait Body: Stream + Sized {
    type Chunk: From<::chunk::Chunk>;
    type Error: From<::error::Error>;

    fn empty() -> Self;

    fn pair() -> (mpsc::Sender<Result<Self::Chunk, <Self as Body>::Error>>, Self);
}
