use super::http::{Request, Response};
use super::protocol::enforce_package_size;
use super::{Protocol, ProtocolError};

const MAX_HEADER_SIZE: usize = 16_384;

#[derive(Clone, Copy, Debug, Default)]
pub struct Http;

impl Protocol for Http {
    type Outbound = Response;
    type Inbound = Request;

    fn input(buffer: &[u8], max_package_size: usize) -> Result<Option<usize>, ProtocolError> {
        let Some(header_end) = find_bytes(buffer, b"\r\n\r\n") else {
            if buffer.len() >= MAX_HEADER_SIZE {
                return Err(ProtocolError::PackageTooLarge {
                    actual: buffer.len(),
                    maximum: MAX_HEADER_SIZE,
                });
            }
            return Ok(None);
        };
        if header_end >= MAX_HEADER_SIZE {
            return Err(ProtocolError::PackageTooLarge {
                actual: header_end,
                maximum: MAX_HEADER_SIZE,
            });
        }

        let header = std::str::from_utf8(&buffer[..header_end])
            .map_err(|_| ProtocolError::InvalidData("HTTP header is not valid ASCII/UTF-8"))?;
        let mut lines = header.split("\r\n");
        let request_line = lines
            .next()
            .ok_or(ProtocolError::InvalidData("HTTP request line is missing"))?;
        Request::validate_request_line(request_line)?;

        let mut content_length = None;
        for line in lines {
            let (name, value) = line
                .split_once(':')
                .ok_or(ProtocolError::InvalidData("HTTP header is malformed"))?;
            let name = name.trim();
            let value = value.trim();
            if name.eq_ignore_ascii_case("transfer-encoding") {
                return Err(ProtocolError::InvalidData(
                    "Transfer-Encoding requests are not supported",
                ));
            }
            if name.eq_ignore_ascii_case("content-length") {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| ProtocolError::InvalidNumber)?;
                if content_length
                    .replace(parsed)
                    .is_some_and(|old| old != parsed)
                {
                    return Err(ProtocolError::InvalidData(
                        "conflicting Content-Length headers",
                    ));
                }
            }
        }

        let package_size = header_end
            .checked_add(4)
            .and_then(|size| size.checked_add(content_length.unwrap_or(0)))
            .ok_or(ProtocolError::InvalidNumber)?;
        enforce_package_size(package_size, max_package_size)?;
        Ok(Some(package_size))
    }

    fn encode(data: &Response) -> Result<Vec<u8>, ProtocolError> {
        data.encode()
            .map_err(|_| ProtocolError::InvalidData("failed to read HTTP response file"))
    }

    fn decode(packet: &[u8]) -> Result<Request, ProtocolError> {
        Request::parse(packet)
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}
