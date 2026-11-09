//! std's `net` connecteds: `sendRequest`, over [`crate::web_client`], and
//! `NetResponse`'s getters, as Kotlin Temper's Rust backend has them
//! (`std/net/support.rs`).

use super::promises::Promise;
use super::{Interp, R, Val, fail};
use crate::web_client;
use std::rc::Rc;

/// A `NetResponse`: what a server answered, with its body already read.
#[derive(Debug)]
pub struct NetResponse {
    status: i32,
    content_type: Option<Rc<str>>,
    body: Rc<Promise>,
}

impl Interp<'_> {
    /// `sendRequest(url, method, bodyContent, bodyMimeType)`: a promise
    /// of a `NetResponse`. As in Kotlin Temper's Rust backend, the request
    /// is sent, and waited for, when the run queue reaches it. No response
    /// breaks the promise.
    pub(super) fn send_request(&mut self, args: Vec<Val>) -> R {
        let text = |v: &Val| match v {
            Val::Str(s) => Some(s.to_string()),
            _ => None,
        };
        let [url, method, body, mime_type] = args.as_slice() else {
            return Err(fail("sendRequest takes 4 arguments"));
        };
        let (Some(url), Some(method)) = (text(url), text(method)) else {
            return Err(fail("sendRequest needs a URL and a method"));
        };
        let (body, mime_type) = (text(body), text(mime_type));
        let promise = Rc::new(Promise::default());
        self.queue_job(promise.clone(), move || {
            let response = web_client::send(&url, &method, body.as_deref(), mime_type.as_deref())?;
            let body = response.body.map_or(Val::Null, |b| Val::Str(b.into()));
            Ok(Val::NetResponse(Rc::new(NetResponse {
                status: response.status,
                content_type: response.content_type.map(Rc::from),
                body: Rc::new(Promise::completed(body)),
            })))
        });
        Ok(Val::Promise(promise))
    }
}

/// `NetResponse`'s connected getters.
pub(super) fn property(response: &NetResponse, name: &str) -> Option<Val> {
    Some(match name {
        "status" => Val::Int(response.status),
        "contentType" => response.content_type.clone().map_or(Val::Null, Val::Str),
        "bodyContent" => Val::Promise(response.body.clone()),
        _ => return None,
    })
}
