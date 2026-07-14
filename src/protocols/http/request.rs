use std::collections::BTreeMap;
use std::net::IpAddr;

use crate::protocols::ProtocolError;

pub type Parameters = BTreeMap<String, String>;
pub type Headers = BTreeMap<String, Vec<String>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HttpMethod {
    Get,
    Post,
    Options,
    Head,
    Delete,
    Put,
    Patch,
}

impl HttpMethod {
    pub fn parse(value: &str) -> Result<Self, ProtocolError> {
        match value {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "OPTIONS" => Ok(Self::Options),
            "HEAD" => Ok(Self::Head),
            "DELETE" => Ok(Self::Delete),
            "PUT" => Ok(Self::Put),
            "PATCH" => Ok(Self::Patch),
            _ => Err(ProtocolError::InvalidData("unsupported HTTP method")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Options => "OPTIONS",
            Self::Head => "HEAD",
            Self::Delete => "DELETE",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    buffer: Vec<u8>,
    method: HttpMethod,
    uri: String,
    protocol_version: String,
    headers: Headers,
    query: Parameters,
    form: Parameters,
    cookies: Parameters,
    body_offset: usize,
}

impl Request {
    pub fn parse(buffer: &[u8]) -> Result<Self, ProtocolError> {
        let header_end = find_bytes(buffer, b"\r\n\r\n").ok_or(ProtocolError::UnexpectedEnd)?;
        let raw_head = std::str::from_utf8(&buffer[..header_end])
            .map_err(|_| ProtocolError::InvalidData("HTTP header is not valid ASCII/UTF-8"))?;
        let mut lines = raw_head.split("\r\n");
        let request_line = lines
            .next()
            .ok_or(ProtocolError::InvalidData("HTTP request line is missing"))?;
        let (method, uri, protocol_version) = Self::parse_request_line(request_line)?;
        let headers = parse_headers(lines)?;
        let query = uri
            .split_once('?')
            .map_or_else(Parameters::new, |(_, query)| {
                parse_parameters(query.as_bytes())
            });
        let body_offset = header_end + 4;
        let form = if header_value(&headers, "content-type")
            .is_some_and(|value| value.starts_with("application/x-www-form-urlencoded"))
        {
            parse_parameters(&buffer[body_offset..])
        } else {
            Parameters::new()
        };
        let cookies = header_value(&headers, "cookie")
            .map(parse_cookies)
            .unwrap_or_default();

        Ok(Self {
            buffer: buffer.to_vec(),
            method,
            uri,
            protocol_version,
            headers,
            query,
            form,
            cookies,
            body_offset,
        })
    }

    pub(crate) fn validate_request_line(line: &str) -> Result<(), ProtocolError> {
        Self::parse_request_line(line).map(|_| ())
    }

    fn parse_request_line(line: &str) -> Result<(HttpMethod, String, String), ProtocolError> {
        let mut parts = line.split(' ');
        let method = HttpMethod::parse(
            parts
                .next()
                .ok_or(ProtocolError::InvalidData("HTTP method is missing"))?,
        )?;
        let uri = parts
            .next()
            .filter(|uri| uri.starts_with('/') || *uri == "*")
            .ok_or(ProtocolError::InvalidData("HTTP request target is invalid"))?;
        let version = parts
            .next()
            .and_then(|value| value.strip_prefix("HTTP/"))
            .filter(|version| matches!(*version, "1.0" | "1.1"))
            .ok_or(ProtocolError::InvalidData("HTTP version is unsupported"))?;
        if parts.next().is_some() {
            return Err(ProtocolError::InvalidData(
                "HTTP request line has extra fields",
            ));
        }
        Ok((method, uri.to_owned(), version.to_owned()))
    }

    pub fn method(&self) -> HttpMethod {
        self.method
    }

    pub fn is_method(&self, method: HttpMethod) -> bool {
        self.method == method
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn query_string(&self) -> &str {
        self.uri.split_once('?').map_or("", |(_, query)| query)
    }

    pub fn path(&self) -> &str {
        self.uri.split_once('?').map_or(&self.uri, |(path, _)| path)
    }

    pub fn protocol_version(&self) -> &str {
        &self.protocol_version
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        header_value(&self.headers, name)
    }

    pub fn query(&self) -> &Parameters {
        &self.query
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(String::as_str)
    }

    pub fn form(&self) -> &Parameters {
        &self.form
    }

    pub fn post(&self, name: &str) -> Option<&str> {
        self.form.get(name).map(String::as_str)
    }

    pub fn input(&self, name: &str) -> Option<&str> {
        self.get(name).or_else(|| self.post(name))
    }

    pub fn cookies(&self) -> &Parameters {
        &self.cookies
    }

    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(String::as_str)
    }

    pub fn raw_head(&self) -> &[u8] {
        &self.buffer[..self.body_offset - 4]
    }

    pub fn raw_body(&self) -> &[u8] {
        &self.buffer[self.body_offset..]
    }

    pub fn raw_buffer(&self) -> &[u8] {
        &self.buffer
    }

    pub fn host(&self, without_port: bool) -> Option<&str> {
        let host = self.header("host")?;
        if !without_port {
            return Some(host);
        }
        if host.starts_with('[') {
            return host.find(']').map(|end| &host[..=end]).or(Some(host));
        }
        Some(host.rsplit_once(':').map_or(host, |(name, port)| {
            if port.parse::<u16>().is_ok() {
                name
            } else {
                host
            }
        }))
    }

    pub fn url(&self) -> Option<String> {
        Some(format!("//{}{}", self.host(false)?, self.path()))
    }

    pub fn full_url(&self) -> Option<String> {
        Some(format!("//{}{}", self.host(false)?, self.uri()))
    }

    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .is_some_and(|value| value.contains("/json") || value.contains("+json"))
    }

    pub fn is_html(&self) -> bool {
        self.header("content-type")
            .is_some_and(|value| value.contains("/html") || value.contains("+html"))
    }

    pub fn is_ajax(&self) -> bool {
        self.header("x-requested-with") == Some("XMLHttpRequest")
    }

    pub fn is_pjax(&self) -> bool {
        self.header("x-pjax").is_some_and(|value| !value.is_empty())
    }

    pub fn accepts_any_content_type(&self) -> bool {
        self.accepted_types()
            .any(|media_type| matches!(media_type, "*/*" | "*"))
    }

    pub fn accepts_json(&self) -> bool {
        self.is_json()
            || self
                .accepted_types()
                .any(|media_type| media_type == "application/json" || media_type.ends_with("+json"))
            || self.accepts_any_content_type()
    }

    pub fn expects_json(&self) -> bool {
        (self.is_ajax() && !self.is_pjax()) || (self.accepts_json() && !self.is_html())
    }

    pub fn request_ip(&self) -> Option<IpAddr> {
        const FORWARDED_HEADERS: [&str; 6] = [
            "x-forwarded-for",
            "x-real-ip",
            "client-ip",
            "x-client-ip",
            "remote-addr",
            "via",
        ];
        FORWARDED_HEADERS
            .iter()
            .find_map(|name| self.header(name))
            .and_then(|value| value.split(',').next())
            .and_then(|value| value.trim().parse().ok())
    }

    fn accepted_types(&self) -> impl Iterator<Item = &str> {
        self.header("accept")
            .unwrap_or("")
            .split(',')
            .map(|value| value.split(';').next().unwrap_or("").trim())
            .filter(|value| !value.is_empty())
    }
}

fn parse_headers<'a>(lines: impl Iterator<Item = &'a str>) -> Result<Headers, ProtocolError> {
    let mut headers = Headers::new();
    for line in lines {
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err(ProtocolError::InvalidData(
                "obsolete folded HTTP headers are not supported",
            ));
        }
        let (name, value) = line
            .split_once(':')
            .ok_or(ProtocolError::InvalidData("HTTP header is malformed"))?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            return Err(ProtocolError::InvalidData("HTTP header name is invalid"));
        }
        headers
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(value.trim().to_owned());
    }
    Ok(headers)
}

fn header_value<'a>(headers: &'a Headers, name: &str) -> Option<&'a str> {
    headers
        .get(&name.to_ascii_lowercase())
        .and_then(|values| values.first())
        .map(String::as_str)
}

fn parse_parameters(input: &[u8]) -> Parameters {
    input
        .split(|byte| *byte == b'&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair
                .iter()
                .position(|byte| *byte == b'=')
                .map_or((pair, &[][..]), |position| {
                    (&pair[..position], &pair[position + 1..])
                });
            (url_decode(key), url_decode(value))
        })
        .collect()
}

fn parse_cookies(input: &str) -> Parameters {
    input
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .map(|(name, value)| (url_decode(name.as_bytes()), url_decode(value.as_bytes())))
        .collect()
}

fn url_decode(input: &[u8]) -> String {
    let mut decoded = Vec::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        match input[index] {
            b'+' => decoded.push(b' '),
            b'%' if index + 2 < input.len() => {
                if let (Some(high), Some(low)) = (hex(input[index + 1]), hex(input[index + 2])) {
                    decoded.push((high << 4) | low);
                    index += 2;
                } else {
                    decoded.push(input[index]);
                }
            }
            byte => decoded.push(byte),
        }
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
