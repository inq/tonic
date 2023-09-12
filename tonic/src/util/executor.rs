use bytes::{Bytes, Buf};
use http_body::Body;

use crate::{transport::{TokioExec, LocalExec}, Status, body::{empty_body, local_empty_body}};

pub trait HasBoxBody {
    type BoxBody: Body<Data = Bytes, Error = crate::Status> + Unpin;

    fn empty_body() -> Self::BoxBody;
}

impl HasBoxBody for TokioExec {
    type BoxBody = crate::body::BoxBody;

    fn empty_body() -> Self::BoxBody {
        empty_body()
    }
}

impl HasBoxBody for LocalExec {
    type BoxBody = crate::body::LocalBoxBody;

    fn empty_body() -> Self::BoxBody {
        local_empty_body()
    }
}

pub trait MakeBoxBody<B>: MaybeSend<B> {
    fn make_box_body(body: B) -> Self::BoxBody;

    fn copy_to_box_body(body: B) -> Self::BoxBody;
}

impl<B> MakeBoxBody<B> for TokioExec
where
    B: Body<Data = Bytes, Error = Status> + Send + 'static,
    B::Error: Into<crate::Error>,
{
    fn make_box_body(body: B) -> Self::BoxBody {
        Self::BoxBody::new(body)
    }

    fn copy_to_box_body(body: B) -> Self::BoxBody {
        Self::BoxBody::new(
            body
                .map_data(|mut buf| buf.copy_to_bytes(buf.remaining()))
                .map_err(|err| Status::map_error(crate::Error::from(err)))
        )
    }

}

impl<B> MakeBoxBody<B> for LocalExec
where
    B: Body<Data = Bytes, Error = Status> + 'static,
    B::Error: Into<crate::Error>,
{
    fn make_box_body(body: B) -> Self::BoxBody {
        Self::BoxBody::new(body)
    }

    fn copy_to_box_body(body: B) -> Self::BoxBody {
        Self::BoxBody::new(
            body
                .map_data(|mut buf| buf.copy_to_bytes(buf.remaining()))
                .map_err(|err| Status::map_error(crate::Error::from(err)))
        )
    }
}

pub trait Boxed<B>: MaybeSend<B> + HasBoxBody {
    /// Convert a [`http_body::Body`] into a [`BoxBody`].
    fn boxed(body: B) -> Self::BoxBody;
}

impl<B> Boxed<B> for TokioExec
where
    B: Body<Data = Bytes> + Send + 'static,
    B::Error: Into<crate::Error>,
{
    fn boxed(body: B) -> Self::BoxBody {
        Self::BoxBody::new(body.map_err(crate::Status::map_error))
    }
}

impl<B> Boxed<B> for LocalExec
where
    B: Body<Data = Bytes> + 'static,
    B::Error: Into<crate::Error>,
{
    fn boxed(body: B) -> Self::BoxBody {
        Self::BoxBody::new(body.map_err(crate::Status::map_error))
    }
}

pub trait MaybeSend<B>: HasBoxBody {
}

impl<B> MaybeSend<B> for TokioExec
where
    B: Send,
{

}

impl<B> MaybeSend<B> for LocalExec
{

}
