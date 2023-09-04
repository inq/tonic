//! HTTP specific body utilities.

use std::convert::Infallible;
use std::fmt;
use std::pin::Pin;

use bytes::Buf;
use http::{Request, Response};
use http_body::{Body, SizeHint, Empty};
use tower::util::BoxCloneService;

use crate::util::LocalBoxCloneService;

pub fn empty_body() -> BoxBody {
    http_body::Empty::new()
        .map_err(|err| match err {})
        .boxed_unsync()
}

pub trait BoxBodyExt: Body<Data = bytes::Bytes, Error = crate::Status> + 'static {
    type BoxCloneService;

    fn empty_body() -> Self;
}

/// A type erased HTTP body used for tonic services.
pub type BoxBody = http_body::combinators::UnsyncBoxBody<bytes::Bytes, crate::Status>;

/// Convert a [`http_body::Body`] into a [`BoxBody`].
pub(crate) fn boxed<B>(body: B) -> BoxBody
where
    B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<crate::Error>,
{
    body.map_err(crate::Status::map_error).boxed_unsync()
}

pub trait IntoBoxBodyExt<T: BoxBodyExt> {
    fn into_box_body(self) -> T;
}

impl<B> IntoBoxBodyExt<BoxBody> for B
where
    B: http_body::Body<Data = bytes::Bytes, Error = crate::Status> + Send + 'static,
{
    fn into_box_body(self) -> BoxBody {
        http_body::combinators::UnsyncBoxBody::new(self)
    }
}

/// Create an empty `BoxBody`
impl BoxBodyExt for BoxBody {
    type BoxCloneService = BoxCloneService<Request<hyper::Body>, Response<Self>, Infallible>;

    fn empty_body() -> Self {
        http_body::Empty::new()
            .map_err(|err| match err {})
            .boxed_unsync()
    }
}

pub type LocalBoxHttpBody = UnsendBoxBody<bytes::Bytes, crate::Error>;
pub type LocalBoxBody = UnsendBoxBody<bytes::Bytes, crate::Status>;

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

/// Convert a [`http_body::Body`] into a [`LocalBoxBody`].
pub(crate) fn local_boxed<B>(body: B) -> LocalBoxBody
where
    B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
    B::Error: Into<crate::Error>,
{
    LocalBoxBody::new(
        body.map_err(crate::Status::map_error)
    )
}

/// Create an empty `BoxBody`
impl BoxBodyExt for LocalBoxBody {
    type BoxCloneService = LocalBoxCloneService<Request<hyper::Body>, Response<Self>, Infallible>;

    fn empty_body() -> Self {
        LocalBoxBody::new(
            http_body::Empty::new()
                .map_err(|err| match err {})
        )
    }
}

impl<B> IntoBoxBodyExt<LocalBoxBody> for B
where
    B: http_body::Body<Data = bytes::Bytes, Error = crate::Status> + 'static,
{
    fn into_box_body(self) -> LocalBoxBody {
        LocalBoxBody::new(self)
    }
}
