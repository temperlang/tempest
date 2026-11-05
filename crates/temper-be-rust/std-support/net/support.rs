//! std `net`'s support code, tempest's own, after Kotlin's: `send_request`
//! sends a request with `ureq` on a thread of its own (`temper-io`), never
//! inside a turn, where Kotlin's runs it as an async block. Settling its
//! promise there queues the waiters on the runner, which waits for it
//! meanwhile (`AsyncRunner::hold`). Without the `net` feature, or a
//! thread, the promise is broken.

use super::*;
use std::sync::Arc;
use temper_core::{Promise, PromiseBuilder, ToArcString};

pub(crate) fn send_request(
    url: &str,
    method: impl ToArcString,
    body_content: Option<Arc<String>>,
    body_mime_type: Option<Arc<String>>,
) -> Promise<NetResponse> {
    let response = PromiseBuilder::new();
    let promise = response.promise();
    #[cfg(feature = "net")]
    {
        let url = url.to_string();
        let method = method.to_arc_string();
        let hold = crate::support::config().runner().hold();
        let settle = response.clone();
        let spawned = std::thread::Builder::new()
            .name("temper-io".into())
            .spawn(move || {
                let _hold = hold;
                let sent = make_request(
                    &url,
                    &method,
                    body_content.as_deref().map(String::as_str),
                    body_mime_type.as_deref().map(String::as_str),
                );
                match sent {
                    Ok(sent) => {
                        let body = PromiseBuilder::new();
                        body.complete(sent.body_content);
                        settle.complete(NetResponse::new(SimpleNetResponse(Arc::new(
                            SimpleNetResponseStruct {
                                status: sent.status,
                                content_type: sent.content_type,
                                body_content: body.promise(),
                            },
                        ))));
                    }
                    Err(_) => settle.break_promise(),
                }
            });
        if spawned.is_err() {
            response.break_promise();
        }
    }
    #[cfg(not(feature = "net"))]
    {
        let _ = (url, method.to_arc_string(), body_content, body_mime_type);
        response.break_promise();
    }
    promise
}

#[cfg(feature = "net")]
fn make_request(
    url: &str,
    method: &str,
    body_content: Option<&str>,
    body_mime_type: Option<&str>,
) -> Result<SimplerNetResponse, ureq::Error> {
    use ureq::http;
    let request = http::Request::builder().uri(url).method(method);
    let request = match body_mime_type {
        Some(body_mime_type) => request.header("Content-Type", body_mime_type),
        _ => request,
    };
    // By default ureq turns a 4xx or 5xx status into an error, which
    // would break the promise. An HTTP error status is still a response,
    // so it resolves with it, as JS and Java do.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = match body_content {
        Some(body_content) => agent.run(request.body(body_content)?),
        None => agent.run(request.body(())?),
    }?;
    let content_type = match response.headers().get(http::header::CONTENT_TYPE) {
        Some(value) => Some(
            value
                .to_str()
                .map_err(|e| ureq::Error::Other(Box::new(e)))?
                .to_arc_string(),
        ),
        None => None,
    };
    Ok(SimplerNetResponse {
        status: i32::from(response.status().as_u16()),
        content_type,
        body_content: (response.body_mut().read_to_string().ok()).map(|x| x.to_arc_string()),
    })
}

#[cfg(feature = "net")]
struct SimplerNetResponse {
    status: i32,
    content_type: Option<Arc<String>>,
    body_content: Option<Arc<String>>,
}

struct SimpleNetResponseStruct {
    status: i32,
    content_type: Option<Arc<String>>,
    body_content: Promise<Option<Arc<String>>>,
}

#[derive(Clone)]
struct SimpleNetResponse(Arc<SimpleNetResponseStruct>);

impl NetResponseTrait for SimpleNetResponse {
    fn clone_boxed(&self) -> NetResponse {
        NetResponse::new(self.clone())
    }

    fn status(&self) -> i32 {
        self.0.status
    }

    fn content_type(&self) -> Option<Arc<String>> {
        self.0.content_type.clone()
    }

    fn body_content(&self) -> Promise<Option<Arc<String>>> {
        self.0.body_content.clone()
    }
}

temper_core::impl_any_value_trait!(SimpleNetResponse, [NetResponse]);
