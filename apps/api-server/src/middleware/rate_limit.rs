//! Rate limiting middleware.

use actix_web::{
    Error, HttpResponse,
    body::EitherBody,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use apex_shared::ErrorResponse;
use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;

use apex_core::ports::RateLimiter;

/// Rate limiting middleware factory.
pub struct RateLimitMiddleware {
    limiter: Arc<dyn RateLimiter>,
}

impl RateLimitMiddleware {
    pub fn new(limiter: Arc<dyn RateLimiter>) -> Self {
        Self { limiter }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimitMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Transform = RateLimitMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimitMiddlewareService {
            service,
            limiter: self.limiter.clone(),
        }))
    }
}

pub struct RateLimitMiddlewareService<S> {
    service: S,
    limiter: Arc<dyn RateLimiter>,
}

impl<S, B> Service<ServiceRequest> for RateLimitMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let limiter = self.limiter.clone();

        // Get client identifier (IP address or user ID)
        let key = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();

        // Check rate limit synchronously before calling inner service
        // We need to check first, then either proceed or reject
        let check_result = {
            // Use block_on for the sync check - in production, consider using
            // a different approach or the keyed rate limiter that's sync
            futures::executor::block_on(limiter.check(&key))
        };

        match check_result {
            Ok(result) if !result.allowed => {
                // Rate limited - return 429 immediately
                tracing::warn!("Rate limit exceeded for key: {}", key);

                let error = ErrorResponse::new(429, "Too Many Requests").with_detail(format!(
                    "Rate limit exceeded. Try again in {} seconds.",
                    result.reset_after.as_secs()
                ));

                let response = HttpResponse::TooManyRequests()
                    .insert_header(("X-RateLimit-Remaining", "0"))
                    .insert_header(("Retry-After", result.reset_after.as_secs().to_string()))
                    .json(error);

                let (http_req, _payload) = req.into_parts();
                let srv_response = ServiceResponse::new(http_req, response);

                Box::pin(async move { Ok(srv_response.map_into_right_body()) })
            }
            Ok(_) | Err(_) => {
                // Allowed or error (fail open) - proceed with request
                if check_result.is_err() {
                    tracing::error!("Rate limiter error, failing open");
                }

                let fut = self.service.call(req);
                Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_left_body())
                })
            }
        }
    }
}
