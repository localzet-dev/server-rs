# Карта переноса localzet/Server

Исходник: `D:\DevDrive\localzet\Server`.

Статусы:

- `готово` — поведение перенесено и покрыто тестами;
- `частично` — существует рабочий Rust-эквивалент, но API или поведение исходника перенесены не полностью;
- `ожидает` — перенос файла ещё не начат.

| PHP-файл | Rust-файл | Статус |
|---|---|---|
| `Helpers.php` | — | ожидает |
| `Server.php` | `src/server.rs` | частично |
| `ServerAbstract.php` | `src/server_abstract.rs` | частично |
| `Timer.php` | — | ожидает |
| `src/Connection/ConnectionInterface.php` | `src/connection/base.rs` | частично |
| `src/Connection/TcpConnection.php` | `src/connection/tcp_connection.rs` | частично |
| `src/Connection/UdpConnection.php` | `src/connection/udp_connection.rs` | готово |
| `src/Connection/AsyncTcpConnection.php` | — | ожидает |
| `src/Connection/AsyncUdpConnection.php` | `src/connection/async_udp_connection.rs` | частично |
| `src/Protocols/ProtocolInterface.php` | `src/protocols/protocol.rs` | готово |
| `src/Protocols/Text.php` | `src/protocols/text.rs` | готово |
| `src/Protocols/Frame.php` | `src/protocols/frame.rs` | готово |
| `src/Protocols/Redis.php` | `src/protocols/redis.rs` | готово |
| `src/Protocols/Http.php` | `src/protocols/http_protocol.rs` | частично |
| `src/Protocols/Https.php` | `src/protocols/https.rs` | готово |
| `src/Protocols/Ws.php` | — | ожидает |
| `src/Protocols/Websocket.php` | — | ожидает |
| `src/Protocols/Http/Chunk.php` | `src/protocols/http/chunk.rs` | готово |
| `src/Protocols/Http/Request.php` | `src/protocols/http/request.rs` | частично |
| `src/Protocols/Http/Response.php` | `src/protocols/http/response.rs` | частично |
| `src/Protocols/Http/ServerSentEvents.php` | `src/protocols/http/server_sent_events.rs` | готово |
| `src/Protocols/Http/Session.php` | — | ожидает |
| `src/Protocols/Http/Session/SessionHandlerInterface.php` | — | ожидает |
| `src/Protocols/Http/Session/FileSessionHandler.php` | — | ожидает |
| `src/Protocols/Http/Session/RedisSessionHandler.php` | — | ожидает |
| `src/Protocols/Http/Session/RedisClusterSessionHandler.php` | — | ожидает |
| `src/Protocols/Http/Session/MongoSessionHandler.php` | — | ожидает |
| `src/Events/EventInterface.php` | — | ожидает |
| `src/Events/Event.php` | — | ожидает |
| `src/Events/Ev.php` | — | ожидает |
| `src/Events/Linux.php` | — | ожидает |
| `src/Events/Swoole.php` | — | ожидает |
| `src/Events/Swow.php` | — | ожидает |
| `src/Events/Windows.php` | — | ожидает |
| `src/Events/Linux/CallbackType.php` | — | ожидает |
| `src/Events/Linux/Driver.php` | — | ожидает |
| `src/Events/Linux/DriverFactory.php` | — | ожидает |
| `src/Events/Linux/FiberLocal.php` | — | ожидает |
| `src/Events/Linux/InvalidCallbackError.php` | — | ожидает |
| `src/Events/Linux/Suspension.php` | — | ожидает |
| `src/Events/Linux/UncaughtThrowable.php` | — | ожидает |
| `src/Events/Linux/UnsupportedFeatureException.php` | — | ожидает |
| `src/Events/Linux/Driver/EvDriver.php` | — | ожидает |
| `src/Events/Linux/Driver/EventDriver.php` | — | ожидает |
| `src/Events/Linux/Driver/StreamSelectDriver.php` | — | ожидает |
| `src/Events/Linux/Driver/TracingDriver.php` | — | ожидает |
| `src/Events/Linux/Driver/UvDriver.php` | — | ожидает |
| `src/Events/Linux/Internal/AbstractDriver.php` | — | ожидает |
| `src/Events/Linux/Internal/ClosureHelper.php` | — | ожидает |
| `src/Events/Linux/Internal/DeferCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/DriverCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/DriverSuspension.php` | — | ожидает |
| `src/Events/Linux/Internal/SignalCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/StreamCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/StreamReadableCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/StreamWritableCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/TimerCallback.php` | — | ожидает |
| `src/Events/Linux/Internal/TimerQueue.php` | — | ожидает |
