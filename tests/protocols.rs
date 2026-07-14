use server_rs::protocols::http::{Chunk, HttpMethod, Request, Response, ServerSentEvents};
use server_rs::protocols::{
    Frame, Http, Protocol, ProtocolError, Redis, RedisArgument, RedisValue, Text,
};

#[test]
fn text_protocol_handles_partial_and_complete_lines() {
    assert_eq!(Text::input(b"hello", 32).unwrap(), None);
    assert_eq!(Text::input(b"hello\r\nnext", 32).unwrap(), Some(7));
    assert_eq!(Text::encode(b"hello").unwrap(), b"hello\n");
    assert_eq!(Text::decode(b"hello\r\n").unwrap(), b"hello");
    assert_eq!(
        Text::input(b"1234", 4),
        Err(ProtocolError::PackageTooLarge {
            actual: 4,
            maximum: 4,
        })
    );
}

#[test]
fn frame_protocol_uses_big_endian_total_length() {
    let frame = Frame::encode(b"payload").unwrap();
    assert_eq!(&frame[..4], &11_u32.to_be_bytes());
    assert_eq!(Frame::input(&frame[..3], 1024).unwrap(), None);
    assert_eq!(Frame::input(&frame, 1024).unwrap(), Some(11));
    assert_eq!(Frame::decode(&frame).unwrap(), b"payload");
}

#[test]
fn redis_protocol_encodes_commands_and_decodes_nested_responses() {
    let command = Redis::encode(&[
        RedisArgument::from("MSET"),
        RedisArgument::Group(vec![
            RedisArgument::from("key"),
            RedisArgument::from("value"),
        ]),
    ])
    .unwrap();
    assert_eq!(command, b"*3\r\n$4\r\nMSET\r\n$3\r\nkey\r\n$5\r\nvalue\r\n");

    let response = b"*3\r\n:1\r\n$5\r\nhello\r\n$-1\r\n";
    assert_eq!(Redis::input(response, 1024).unwrap(), Some(response.len()));
    assert_eq!(
        Redis::decode(response).unwrap(),
        RedisValue::Array(Some(vec![
            RedisValue::Integer(1),
            RedisValue::BulkString(Some(b"hello".to_vec())),
            RedisValue::BulkString(None),
        ]))
    );
}

#[test]
fn http_streaming_value_objects_match_wire_format() {
    assert_eq!(Chunk::new(b"hello".to_vec()).encode(), b"5\r\nhello\r\n");
    let event = ServerSentEvents::new()
        .with_event("ping")
        .with_id("1000")
        .with_retry(5000)
        .with_data("first\nsecond");
    assert_eq!(
        event.encode(),
        "event: ping\nid: 1000\nretry: 5000\ndata: first\ndata: second\n\n"
    );
}

#[test]
fn http_protocol_detects_and_parses_complete_request() {
    let packet = b"POST /users?active=1 HTTP/1.1\r\nHost: example.test:8080\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 15\r\nCookie: sid=abc; theme=dark\r\nAccept: application/json\r\n\r\nname=Ivan+Zorin";
    assert_eq!(Http::input(&packet[..40], 4096).unwrap(), None);
    assert_eq!(Http::input(packet, 4096).unwrap(), Some(packet.len()));

    let request = Http::decode(packet).unwrap();
    assert_eq!(request.method(), HttpMethod::Post);
    assert_eq!(request.path(), "/users");
    assert_eq!(request.get("active"), Some("1"));
    assert_eq!(request.post("name"), Some("Ivan Zorin"));
    assert_eq!(request.cookie("theme"), Some("dark"));
    assert_eq!(request.host(true), Some("example.test"));
    assert!(request.expects_json());
}

#[test]
fn http_protocol_rejects_request_smuggling_inputs() {
    let conflicting = b"POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\na";
    assert!(matches!(
        Http::input(conflicting, 4096),
        Err(ProtocolError::InvalidData(_))
    ));
    let chunked = b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
    assert!(matches!(
        Http::input(chunked, 4096),
        Err(ProtocolError::InvalidData(_))
    ));
}

#[test]
fn http_response_formats_headers_body_and_cookies() {
    let response = Response::new(201)
        .with_header("content-type", "text/plain")
        .with_cookie(
            "token",
            "a value",
            Some(60),
            Some("/"),
            None,
            true,
            true,
            Some("Lax"),
        )
        .with_body(b"created".to_vec());
    let encoded = Http::encode(&response).unwrap();
    let encoded = String::from_utf8(encoded).unwrap();
    assert!(encoded.starts_with("HTTP/1.1 201 Created\r\n"));
    assert!(encoded.contains("content-length: 7\r\n"));
    assert!(encoded.contains(
        "set-cookie: token=a%20value; Max-Age=60; Path=/; Secure; HttpOnly; SameSite=Lax\r\n"
    ));
    assert!(encoded.ends_with("\r\n\r\ncreated"));
}

#[test]
fn request_parser_rejects_unsupported_methods() {
    let packet = b"TRACE / HTTP/1.1\r\nHost: example.test\r\n\r\n";
    assert!(matches!(
        Request::parse(packet),
        Err(ProtocolError::InvalidData("unsupported HTTP method"))
    ));
}
