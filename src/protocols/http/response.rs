use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub type Headers = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileBody {
    path: PathBuf,
    offset: u64,
    length: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Response {
    status: u16,
    headers: Headers,
    reason: Option<String>,
    version: String,
    body: Vec<u8>,
    file: Option<FileBody>,
}

impl Default for Response {
    fn default() -> Self {
        Self::new(200)
    }
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Headers::new(),
            reason: None,
            version: "1.1".to_owned(),
            body: Vec::new(),
            file: None,
        }
    }

    pub fn with_header(mut self, name: impl AsRef<str>, value: impl Into<String>) -> Self {
        self.headers
            .insert(name.as_ref().to_ascii_lowercase(), vec![value.into()]);
        self
    }

    pub fn append_header(mut self, name: impl AsRef<str>, value: impl Into<String>) -> Self {
        self.headers
            .entry(name.as_ref().to_ascii_lowercase())
            .or_default()
            .push(value.into());
        self
    }

    pub fn without_header(mut self, name: &str) -> Self {
        self.headers.remove(&name.to_ascii_lowercase());
        self
    }

    pub fn header(&self, name: &str) -> Option<&[String]> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(Vec::as_slice)
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub fn with_protocol_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self.file = None;
        self
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }

    pub fn with_status(mut self, status: u16, reason: Option<impl Into<String>>) -> Self {
        self.status = status;
        self.reason = reason.map(Into::into);
        self
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, offset: u64, length: Option<u64>) -> Self {
        self.file = Some(FileBody {
            path: path.into(),
            offset,
            length,
        });
        self.body.clear();
        self
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_cookie(
        self,
        name: &str,
        value: &str,
        max_age: Option<u64>,
        path: Option<&str>,
        domain: Option<&str>,
        secure: bool,
        http_only: bool,
        same_site: Option<&str>,
    ) -> Self {
        let mut cookie = format!("{}={}", name, percent_encode(value.as_bytes()));
        if let Some(domain) = domain {
            cookie.push_str("; Domain=");
            cookie.push_str(domain);
        }
        if let Some(max_age) = max_age {
            cookie.push_str(&format!("; Max-Age={max_age}"));
        }
        if let Some(path) = path {
            cookie.push_str("; Path=");
            cookie.push_str(path);
        }
        if secure {
            cookie.push_str("; Secure");
        }
        if http_only {
            cookie.push_str("; HttpOnly");
        }
        if let Some(same_site) = same_site {
            cookie.push_str("; SameSite=");
            cookie.push_str(same_site);
        }
        self.append_header("set-cookie", cookie)
    }

    pub fn encode(&self) -> io::Result<Vec<u8>> {
        let mut headers = self.headers.clone();
        headers
            .entry("server".to_owned())
            .or_insert_with(|| vec!["Localzet-Server".to_owned()]);
        headers
            .entry("connection".to_owned())
            .or_insert_with(|| vec!["keep-alive".to_owned()]);

        let body = if let Some(file) = &self.file {
            read_file_body(file, &mut headers)?
        } else {
            headers
                .entry("content-type".to_owned())
                .or_insert_with(|| vec!["text/html;charset=utf-8".to_owned()]);
            self.body.clone()
        };
        if !headers.contains_key("transfer-encoding") && !body.is_empty() {
            headers.insert("content-length".to_owned(), vec![body.len().to_string()]);
        }

        let reason = self
            .reason
            .as_deref()
            .unwrap_or_else(|| reason_phrase(self.status));
        let mut encoded =
            format!("HTTP/{} {} {}\r\n", self.version, self.status, reason).into_bytes();
        for (name, values) in headers {
            for value in values.into_iter().filter(|value| !value.is_empty()) {
                encoded.extend_from_slice(name.as_bytes());
                encoded.extend_from_slice(b": ");
                encoded.extend_from_slice(value.as_bytes());
                encoded.extend_from_slice(b"\r\n");
            }
        }
        encoded.extend_from_slice(b"\r\n");

        if self.version == "1.1"
            && self
                .header("transfer-encoding")
                .is_some_and(|values| values.iter().any(|value| value == "chunked"))
            && !body.is_empty()
        {
            encoded.extend_from_slice(format!("{:x}\r\n", body.len()).as_bytes());
            encoded.extend_from_slice(&body);
            encoded.extend_from_slice(b"\r\n");
        } else {
            encoded.extend_from_slice(&body);
        }
        Ok(encoded)
    }
}

fn read_file_body(file: &FileBody, headers: &mut Headers) -> io::Result<Vec<u8>> {
    let content = fs::read(&file.path)?;
    let offset = usize::try_from(file.offset)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "file offset is too large"))?;
    if offset >= content.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "file offset is outside the file",
        ));
    }
    let requested = file.length.map_or(content.len() - offset, |length| {
        usize::try_from(length).unwrap_or(usize::MAX)
    });
    let end = offset.saturating_add(requested).min(content.len());
    let body = content[offset..end].to_vec();
    headers.insert("accept-ranges".to_owned(), vec!["bytes".to_owned()]);
    headers.insert("content-length".to_owned(), vec![body.len().to_string()]);
    headers.entry("content-type".to_owned()).or_insert_with(|| {
        vec![
            mime_type(&file.path)
                .unwrap_or("application/octet-stream")
                .to_owned(),
        ]
    });
    if offset > 0 || file.length.is_some() {
        headers.insert(
            "content-range".to_owned(),
            vec![format!("bytes {}-{}/{}", offset, end - 1, content.len())],
        );
    }
    Ok(body)
}

fn mime_type(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "css" => Some("text/css"),
        "gif" => Some("image/gif"),
        "htm" | "html" => Some("text/html"),
        "ico" => Some("image/x-icon"),
        "jpeg" | "jpg" => Some("image/jpeg"),
        "js" => Some("text/javascript"),
        "json" => Some("application/json"),
        "png" => Some("image/png"),
        "svg" => Some("image/svg+xml"),
        "txt" => Some("text/plain"),
        "webp" => Some("image/webp"),
        "xml" => Some("application/xml"),
        _ => None,
    }
}

fn percent_encode(value: &[u8]) -> String {
    let mut encoded = String::new();
    for byte in value {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(*byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        418 => "I'm a teapot",
        422 => "Unprocessable Entity",
        426 => "Upgrade Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        _ => "Unknown Status",
    }
}
