use server_rs::protocols::http::{Chunk, ServerSentEvents};
use server_rs::protocols::{
    Frame, Protocol, ProtocolError, Redis, RedisArgument, RedisValue, Text,
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
