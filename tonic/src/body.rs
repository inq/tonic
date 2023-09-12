//! HTTP specific body utilities.

use std::fmt;
use std::pin::Pin;

use bytes::{Buf, Bytes};
use http_body::{Body, SizeHint, Empty};

/// A type erased HTTP body used for tonic services.
pub type BoxBody = http_body::combinators::UnsyncBoxBody<bytes::Bytes, crate::Status>;
pub type LocalBoxBody = UnsendBoxBody<bytes::Bytes, crate::Status>;

pub type BoxHttpBody = http_body::combinators::UnsyncBoxBody<Bytes, crate::Error>;
pub type LocalBoxHttpBody = crate::body::UnsendBoxBody<Bytes, crate::Error>;

pub fn empty_body() -> BoxBody {
    http_body::Empty::new()
        .map_err(|err| match err {})
        .boxed_unsync()
}
pub fn local_empty_body() -> LocalBoxBody {
    LocalBoxBody::new(
        http_body::Empty::new()
            .map_err(|err| match err {})
    )
}

pub struct UnsendBoxBody<D, E> {
    inner: Pin<Box<dyn Body<Data = D, Error = E> + 'static>>,
}

impl<D, E> UnsendBoxBody<D, E> {
    /// Create a new `BoxBody`.
    pub fn new<B>(body: B) -> Self
    where
        B: Body<Data = D, Error = E> + 'static,
        D: Buf,
    {
        Self {
            inner: Box::pin(body),
        }
    }
}

impl<D, E> fmt::Debug for UnsendBoxBody<D, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UnsyncBoxBody").finish()
    }
}

impl<D, E> Body for UnsendBoxBody<D, E>
where
    D: Buf,
{
    type Data = D;
    type Error = E;

    fn poll_data(
            mut self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Result<Self::Data, Self::Error>>> {
        self.inner.as_mut().poll_data(cx)
    }

    fn poll_trailers(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        self.inner.as_mut().poll_trailers(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

impl<D, E> Default for UnsendBoxBody<D, E>
where
    D: Buf + 'static,
{
    fn default() -> Self {
        UnsendBoxBody::new(Empty::new().map_err(|err| match err {}))
    }
}
