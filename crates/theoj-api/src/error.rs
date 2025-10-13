use axum::{http::StatusCode, response::IntoResponse};
use serde::Serialize;
use serde_json::json;
use std::fmt::{Debug, Display};
use tracing::error;
use utoipa::ToSchema;

#[macro_export]
macro_rules! bail {
    ($fmt:expr) => {
        $crate::bail!(@BAD_REQUEST $fmt)
    };
    ($fmt:expr, $($t:tt)*) => {
        $crate::bail!(@BAD_REQUEST $fmt, $($t)*)
    };
    (@$code:ident $str:expr) => {
        return $crate::Result::Err($crate::Error(axum::http::StatusCode::$code, Some(anyhow::Error::msg($str))))
    };
    (@$code:ident $fmt:expr, $($args:tt)*) => {
        return $crate::Result::Err($crate::Error(axum::http::StatusCode::$code, Some(anyhow::anyhow!($fmt, $($args)*))))
    };
    (@$code:ident) => {
        return $crate::Result::Err($crate::Error(axum::http::StatusCode::$code, None))
    };
}

#[derive(Debug)]
pub struct Error(pub StatusCode, pub Option<anyhow::Error>);

impl Error {
    pub fn msg<S: Into<String>>(s: S) -> Self {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(anyhow::Error::msg(s.into())),
        )
    }

    pub fn anyhow(e: anyhow::Error) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, Some(e))
    }

    pub fn context<C>(self, context: C) -> Self
    where
        C: Display + Debug + Send + Sync + 'static,
    {
        Self(
            StatusCode::INTERNAL_SERVER_ERROR,
            Some(match self.1 {
                Some(err) => err.context(context),
                None => anyhow::Error::msg(context),
            }),
        )
    }

    pub fn status_code(self, code: StatusCode) -> Self {
        Self(code, self.1)
    }
}

impl<E> From<E> for Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(value: E) -> Self {
        Self(StatusCode::INTERNAL_SERVER_ERROR, Some(value.into()))
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        if self.0 == StatusCode::INTERNAL_SERVER_ERROR {
            error!("request failed with error: {:?}", self.1);
        }
        (
            self.0,
            self.1
                .map(|it| json!({ "message": format!("{it}") }).to_string())
                .unwrap_or_default(),
        )
            .into_response()
    }
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub message: String,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
