//! Request ID middleware - generates unique IDs for each request.

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
    http::header::{HeaderName, HeaderValue},
};
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use uuid::Uuid;

/// Header name for request ID.
pub static REQUEST_ID_HEADER: &str = "X-Request-ID";

/// Middleware that generates a unique request ID for each request.
/// The ID is added to response headers and available in tracing spans.
pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestIdService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdService { service }))
    }
}

pub struct RequestIdService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Check if request already has an ID (from client or load balancer)
        let request_id = req
            .headers()
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Store request ID in extensions for handlers to access
        req.extensions_mut().insert(RequestId(request_id.clone()));

        // Add request ID to tracing span
        let span = tracing::info_span!(
            "request",
            request_id = %request_id,
        );
        let _guard = span.enter();

        tracing::debug!("Processing request with ID: {}", request_id);

        let fut = self.service.call(req);
        let request_id_header = request_id.clone();

        Box::pin(async move {
            let mut res = fut.await?;

            // Add request ID to response headers
            res.headers_mut().insert(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str(&request_id_header)
                    .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
            );

            Ok(res)
        })
    }
}

/// Request ID extractor for handlers.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Extractor to get request ID in handlers.
impl actix_web::FromRequest for RequestId {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &actix_web::HttpRequest, _: &mut actix_web::dev::Payload) -> Self::Future {
        let request_id = req
            .extensions()
            .get::<RequestId>()
            .cloned()
            .unwrap_or_else(|| RequestId(Uuid::new_v4().to_string()));

        ready(Ok(request_id))
    }
}
