//! A web client for std's `net`, the interpreter's `sendRequest`. It
//! knows nothing of the interpreter, so it can move to connected code
//! later. With the `net` feature it uses `ureq`, as Kotlin Temper's Rust
//! backend does; without it, as in the wasm build, every request fails.

/// What a server answered. An HTTP error status is still a response.
#[derive(Debug)]
pub struct Response {
    pub status: i32,
    pub content_type: Option<String>,
    /// The body, if it's text (UTF-8).
    pub body: Option<String>,
}

/// Sends a request and waits for its response, or says why there was none.
#[cfg(feature = "net")]
pub fn send(
    url: &str,
    method: &str,
    body: Option<&str>,
    mime_type: Option<&str>,
) -> Result<Response, String> {
    use ureq::http;
    let mut request = http::Request::builder().uri(url).method(method);
    if let Some(mime_type) = mime_type {
        request = request.header(http::header::CONTENT_TYPE, mime_type);
    }
    // ureq would make a 4xx or 5xx status an error, as no backend does.
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let sent = match body {
        Some(body) => request.body(body).map(|r| agent.run(r)),
        None => request.body(()).map(|r| agent.run(r)),
    };
    let mut response = sent
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let content_type = response
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    Ok(Response {
        status: response.status().as_u16().into(),
        content_type,
        body: response.body_mut().read_to_string().ok(),
    })
}

#[cfg(not(feature = "net"))]
pub fn send(
    _url: &str,
    _method: &str,
    _body: Option<&str>,
    _mime_type: Option<&str>,
) -> Result<Response, String> {
    Err("web client not supported here".into())
}
