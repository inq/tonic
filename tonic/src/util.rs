//! Various utilities used throughout tonic.

// some combinations of features might cause things here not to be used
#![allow(dead_code)]

use std::fmt;
use std::task::{Context, Poll};

use tower_layer::{LayerFn, layer_fn};
use tower_service::Service;
use tower::ServiceExt;

use crate::codegen::LocalBoxFuture;

pub(crate) mod base64 {
    use base64::{
        alphabet,
        engine::{
            general_purpose::{GeneralPurpose, GeneralPurposeConfig},
            DecodePaddingMode,
        },
    };

    pub(crate) const STANDARD: GeneralPurpose = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new()
            .with_encode_padding(true)
            .with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );

    pub(crate) const STANDARD_NO_PAD: GeneralPurpose = GeneralPurpose::new(
        &alphabet::STANDARD,
        GeneralPurposeConfig::new()
            .with_encode_padding(false)
            .with_decode_padding_mode(DecodePaddingMode::Indifferent),
    );
}

pub struct LocalBoxCloneService<T, U, E>(
    Box<
        dyn CloneService<T, Response = U, Error = E, Future = LocalBoxFuture<U, E>>
            + Send,
    >,
);

impl<T, U, E> LocalBoxCloneService<T, U, E> {
    /// Create a new `BoxCloneService`.
    pub fn new<S>(inner: S) -> Self
    where
        S: Service<T, Response = U, Error = E> + Clone + Send + 'static,
        S::Future: 'static,
    {
        let inner = inner.map_future(|f| Box::pin(f) as _);
        Self(Box::new(inner))
    }

    /// Returns a [`Layer`] for wrapping a [`Service`] in a [`BoxCloneService`]
    /// middleware.
    ///
    /// [`Layer`]: crate::Layer
    pub fn layer<S>() -> LayerFn<fn(S) -> Self>
    where
        S: Service<T, Response = U, Error = E> + Clone + Send + 'static,
        S::Future: Send + 'static,
    {
        layer_fn(Self::new)
    }
}

impl<T, U, E> Service<T> for LocalBoxCloneService<T, U, E> {
    type Response = U;
    type Error = E;
    type Future = LocalBoxFuture<U, E>;

    #[inline]
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        self.0.poll_ready(cx)
    }

    #[inline]
    fn call(&mut self, request: T) -> Self::Future {
        self.0.call(request)
    }
}

impl<T, U, E> Clone for LocalBoxCloneService<T, U, E> {
    fn clone(&self) -> Self {
        Self(self.0.clone_box())
    }
}

trait CloneService<R>: Service<R> {
    fn clone_box(
        &self,
    ) -> Box<
        dyn CloneService<R, Response = Self::Response, Error = Self::Error, Future = Self::Future>
            + Send,
    >;
}

impl<R, T> CloneService<R> for T
where
    T: Service<R> + Send + Clone + 'static,
{
    fn clone_box(
        &self,
    ) -> Box<dyn CloneService<R, Response = T::Response, Error = T::Error, Future = T::Future> + Send>
    {
        Box::new(self.clone())
    }
}



/// A boxed `Service + Send` trait object.
///
/// [`BoxService`] turns a service into a trait object, allowing the response
/// future type to be dynamic. This type requires both the service and the
/// response future to be [`Send`].
///
/// If you need a boxed [`Service`] that implements [`Clone`] consider using
/// [`BoxCloneService`](crate::util::BoxCloneService).
///
/// See module level documentation for more details.
pub struct LocalBoxService<T, U, E> {
    inner: Box<dyn Service<T, Response = U, Error = E, Future = LocalBoxFuture<U, E>> + Send>,
}

/// A boxed `Future + Send` trait object.
///
/// This type alias represents a boxed future that is [`Send`] and can be moved
/// across threads.

impl<T, U, E> LocalBoxService<T, U, E> {
    #[allow(missing_docs)]
    pub fn new<S>(inner: S) -> Self
    where
        S: Service<T, Response = U, Error = E> + Send + 'static,
        S::Future: 'static,
    {
        let inner = Box::new(inner.map_future(|f: S::Future| Box::pin(f) as _));
        LocalBoxService { inner }
    }

    /// Returns a [`Layer`] for wrapping a [`Service`] in a [`BoxService`]
    /// middleware.
    ///
    /// [`Layer`]: crate::Layer
    pub fn layer<S>() -> LayerFn<fn(S) -> Self>
    where
        S: Service<T, Response = U, Error = E> + Send + 'static,
        S::Future: 'static,
    {
        layer_fn(Self::new)
    }
}

impl<T, U, E> Service<T> for LocalBoxService<T, U, E> {
    type Response = U;
    type Error = E;
    type Future = LocalBoxFuture<U, E>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: T) -> Self::Future {
        self.inner.call(request)
    }
}

impl<T, U, E> fmt::Debug for LocalBoxService<T, U, E> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("BoxService").finish()
    }
}
