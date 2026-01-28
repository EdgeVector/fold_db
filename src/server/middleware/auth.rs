use std::future::{ready, Ready};

use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use std::rc::Rc;

use crate::logging::core::run_with_user;

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct UserContextMiddleware;

// Middleware factory is `Transform` trait
impl<S, B> Transform<S, ServiceRequest> for UserContextMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = UserContextMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(UserContextMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct UserContextMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for UserContextMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();

        // Extract user hash from headers - check x-user-hash first (primary, matches Lambda)
        // then fall back to x-user-id for backwards compatibility
        let user_id = req
            .headers()
            .get("x-user-hash")
            .or_else(|| req.headers().get("x-user-id"))
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            if let Some(uid) = user_id {
                // Run the next service in the user context
                run_with_user(&uid, async move { svc.call(req).await }).await
            } else {
                // No user context - request will proceed without user identity.
                // API routes use require_user_context() helper to return 401 if needed.
                // Static files and health checks can proceed without authentication.
                // This middleware only PROPAGATES context, it doesn't ENFORCE it.
                svc.call(req).await
            }
        })
    }
}
