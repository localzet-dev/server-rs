# Rust для разработчика на PHP, Python и TypeScript

Этот текст — не справочник по каждому оператору. Его цель — перестроить модель мышления разработчика с опытом PHP, Python и TypeScript и показать Rust на примере сетевого сервера.

## 1. Главное различие — не синтаксис

PHP, Python и обычный TypeScript-код живут в среде выполнения:

- PHP управляет значениями, ссылками и памятью внутри процесса интерпретатора;
- Python использует подсчёт ссылок и сборщик циклического мусора;
- TypeScript после проверки типов превращается в JavaScript и подчиняется модели выполнения JS;
- ошибки владения объектом или гонки часто обнаруживаются только во время работы.

Rust компилируется в нативный код и не использует сборщик мусора. Компилятор должен заранее доказать, что:

- ссылка не переживёт значение, на которое указывает;
- значение не будет использовано после освобождения;
- изменяемый доступ не пересечётся с другим доступом опасным образом;
- данные между потоками передаются безопасно;
- все варианты `enum` обработаны.

Поэтому Rust часто ощущается не как «ещё один язык с фигурными скобками», а как статический анализатор архитектуры, который заодно генерирует машинный код.

Ключевой обмен:

| PHP, Python, TypeScript | Rust |
|---|---|
| Быстро написать первую версию | Нужно раньше определить владельцев данных |
| Многие ошибки проявляются в runtime | Многие классы ошибок запрещены компилятором |
| Память освобождает runtime | Память освобождается детерминированно по правилам владения |
| `null`, исключения и динамические проверки | `Option`, `Result`, `enum` и исчерпывающий `match` |
| Классы и наследование | Структуры, traits и композиция |
| Объекты обычно передаются как ссылки | Передача может переместить, скопировать или одолжить значение |

## 2. От исходника до программы

Основные единицы Rust:

- **package** — пакет с `Cargo.toml`;
- **crate** — единица компиляции: библиотека или бинарная программа;
- **module** — пространство имён внутри crate;
- **item** — функция, структура, trait, константа и другая декларация.

В этом проекте:

```text
Cargo.toml
src/
  lib.rs                 библиотечный crate server_rs
  main.rs                бинарный crate server-rs
  server.rs              модуль server
  server_abstract.rs     модуль server_abstract
  connection/
    mod.rs               корень модуля connection
    base.rs
    tcp_connection.rs
tests/
  tcp_server.rs          интеграционные тесты
```

`src/lib.rs` объявляет внутренние модули и формирует публичный API:

```rust
pub mod connection;
mod server;

pub use server::{Server, ServerConfig};
```

`mod server` подключает модуль, а `pub use` реэкспортирует выбранные типы. Благодаря этому пользователь пишет `server_rs::Server`, а не знает внутренний путь `server_rs::server::Server`.

В отличие от PSR-4, имя файла не обязано повторять имя типа. Принятый стиль:

- типы: `UpperCamelCase`;
- функции, методы, переменные и файлы: `snake_case`;
- константы: `SCREAMING_SNAKE_CASE`.

## 3. Переменные и неизменяемость

В Rust значения неизменяемы по умолчанию:

```rust
let address = "127.0.0.1:8080";
let mut connections = 0;
connections += 1;
```

`mut` — часть контракта. Она сообщает читателю и компилятору, что значение действительно меняется.

Это отличается от `const`:

```rust
const DEFAULT_BUFFER_SIZE: usize = 87_380;
let mut buffer_size = DEFAULT_BUFFER_SIZE;
```

`const` вычисляется в допустимом compile-time контексте и не имеет собственного изменяемого места хранения. `let` создаёт локальную привязку.

Полезна возможность затенения:

```rust
let address = "127.0.0.1:8080";
let address = address.parse::<std::net::SocketAddr>()?;
```

Это новое значение с новым типом, а не изменение старой переменной.

## 4. Владение: центральная идея Rust

У каждого значения есть владелец. Когда владелец покидает область видимости, вызывается `drop`, и ресурсы освобождаются.

```rust
fn consume(value: String) {
    println!("{value}");
}

let name = String::from("echo");
consume(name);
// println!("{name}"); // value was moved
```

`String` владеет памятью в heap. Передача в `consume` перемещает владение. Старую переменную использовать нельзя.

В PHP или Python две переменные обычно могут указывать на один объект, а runtime следит за его жизнью. В Rust совместное владение выражается явно.

### 4.1 Copy и move

Простые типы часто реализуют `Copy`:

```rust
let first = 10_u64;
let second = first;
println!("{first} {second}");
```

Для `String`, `Vec<T>`, файлов и сокетов побитовое неявное копирование было бы опасно, поэтому они перемещаются.

Если действительно нужна независимая копия:

```rust
let first = String::from("echo");
let second = first.clone();
```

`clone()` следует видеть как потенциально дорогую операцию. Не нужно добавлять его автоматически только ради победы над borrow checker.

### 4.2 Заимствование

Функция может временно одолжить значение:

```rust
fn print_name(name: &str) {
    println!("{name}");
}

let name = String::from("echo");
print_name(&name);
println!("{name}");
```

`&T` — неизменяемая ссылка, `&mut T` — изменяемая.

Основное правило в одной области времени:

- либо сколько угодно `&T`;
- либо ровно одна `&mut T`;
- эти режимы не пересекаются.

```rust
let mut data = vec![1, 2, 3];
let first = &data[0];
println!("{first}");
data.push(4);
```

Этот код допустим, потому что последнее использование `first` произошло до `push`. Если использовать `first` после `push`, компилятор запретит код: `push` мог перераспределить память, сделав старую ссылку недействительной.

### 4.3 Владение ресурсом вместо ручной очистки

`TcpConnection` владеет `TcpStream`. При уничтожении соединения срабатывает `Drop`:

```rust
impl Drop for TcpConnection {
    fn drop(&mut self) {
        let _ = self.close();
        self.statistics.connection_closed();
    }
}
```

Это RAII: ресурс связан со временем жизни значения. Тот же подход используется для файлов, mutex guard, транзакций и временных каталогов.

В PHP аналогом по намерению может быть `finally`, но Rust делает очистку структурной и автоматической, включая ранний выход через `?`.

## 5. Ссылки и lifetimes

Lifetime — не таймер и не время в секундах. Это область, в которой ссылка гарантированно действительна.

Большинство lifetime компилятор выводит:

```rust
fn first(values: &[String]) -> Option<&String> {
    values.first()
}
```

Здесь возвращаемая ссылка связана с входным slice. Она не может пережить `values`.

Явная запись нужна при неоднозначности:

```rust
fn choose<'a>(left: &'a str, right: &'a str, use_left: bool) -> &'a str {
    if use_left { left } else { right }
}
```

`'a` не продлевает жизнь данных. Она описывает связь: результат действителен не дольше обоих возможных источников.

Типичная ошибка:

```rust,compile_fail
fn invalid() -> &str {
    let value = String::from("temporary");
    &value
}
```

После выхода из функции `value` уничтожен, поэтому возвращать ссылку нельзя. Нужно вернуть владеющий `String`.

Практическое правило для начала: структуры сервера пусть владеют долгоживущими данными (`String`, `Vec<T>`, `Arc<T>`), а функции принимают короткие заимствования (`&str`, `&[u8]`, `&T`).

## 6. `String`, `&str`, байты и Unicode

У PHP одна строка часто одновременно означает текст и байты. В Rust это разделено:

- `String` — владеющий UTF-8 текст;
- `&str` — заимствованный UTF-8 текст;
- `Vec<u8>` — владеющий буфер байтов;
- `&[u8]` — заимствованный slice байтов.

Сетевой сервер получает байты, а не гарантированный UTF-8:

```rust
fn on_message(&self, connection: &mut dyn Connection, data: &[u8]) -> io::Result<()> {
    connection.send(data)
}
```

Нельзя без проверки считать пакет строкой:

```rust
match std::str::from_utf8(data) {
    Ok(text) => println!("{text}"),
    Err(error) => eprintln!("invalid UTF-8: {error}"),
}
```

Индексировать `String` как `text[0]` нельзя. Один Unicode-символ может занимать несколько байтов, а пользовательская графема — несколько Unicode scalar values.

## 7. Struct и impl вместо класса

Структура хранит данные:

```rust
pub struct ServerConfig {
    name: String,
    address: std::net::SocketAddr,
    read_buffer_size: usize,
}
```

`impl` добавляет методы:

```rust
impl ServerConfig {
    pub fn new(address: std::net::SocketAddr) -> Self {
        Self {
            name: "none".to_owned(),
            address,
            read_buffer_size: 87_380,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}
```

Важны формы receiver:

- `self` — метод забирает объект;
- `&self` — временно читает;
- `&mut self` — временно изменяет;
- `Self` — текущий конкретный тип.

Первоначальная попытка описать события как `fn on_start(self, ...)` забирала сервер при первом вызове. Для повторяемых callbacks обычно нужен `&self` или `&mut self`.

В Rust нет конструкторов как специального элемента языка. `new` — обычная ассоциированная функция по соглашению.

## 8. Traits вместо интерфейсов и наследования

Trait описывает поведение:

```rust
pub trait Connection {
    fn id(&self) -> u64;
    fn send(&mut self, data: &[u8]) -> std::io::Result<()>;
    fn close(&mut self) -> std::io::Result<()>;
}
```

Реализация задаётся отдельно:

```rust
impl Connection for TcpConnection {
    fn id(&self) -> u64 {
        self.id
    }

    fn send(&mut self, data: &[u8]) -> std::io::Result<()> {
        use std::io::Write;
        self.stream.write_all(data)
    }
}
```

Это похоже на PHP interface, но мощнее:

- trait может содержать реализацию методов по умолчанию;
- trait можно реализовать для существующего типа, если соблюдено orphan rule;
- generic-код может требовать набор traits;
- traits участвуют в проверке потокобезопасности (`Send`, `Sync`).

Rust не поддерживает наследование данных. Вместо `TcpConnection extends ConnectionInterface` используется:

- `TcpConnection` хранит собственные данные;
- `impl Connection for TcpConnection` добавляет общий контракт;
- повторное использование строится через композицию и generic-функции.

### 8.1 Статическая и динамическая диспетчеризация

Generic-вариант:

```rust
fn close_connection<C: Connection>(connection: &mut C) -> std::io::Result<()> {
    connection.close()
}
```

Компилятор создаёт специализированный код для конкретного `C`. Это статическая диспетчеризация.

Trait object:

```rust
fn close_connection(connection: &mut dyn Connection) -> std::io::Result<()> {
    connection.close()
}
```

`dyn Connection` хранит ссылку на данные и таблицу методов. Это динамическая диспетчеризация, близкая к вызову метода через interface в PHP.

В сервере `Server<H: ServerHandler>` использует generic для handler, а callback получает `&mut dyn Connection`, чтобы одинаково работать с будущими TCP, UDP или Unix-соединениями.

Не каждый trait совместим с `dyn`. Например, методы, возвращающие неизвестный `Self`, или ассоциированные константы могут сделать trait непригодным как trait object.

## 9. Enum, Option и отсутствие неявного null

Rust enum — алгебраический тип, а не просто число с именем:

```rust
enum Command {
    Start { daemon: bool },
    Stop,
    Reload { graceful: bool },
}
```

Каждый вариант может хранить разные данные.

Отсутствующее значение выражается явно:

```rust
fn find_connection(id: u64) -> Option<TcpConnection> {
    todo!()
}
```

`Option<T>` имеет два варианта:

```rust
enum Option<T> {
    Some(T),
    None,
}
```

Обработка через `match`:

```rust
match find_connection(10) {
    Some(connection) => println!("{}", connection.id()),
    None => println!("not found"),
}
```

Или компактнее:

```rust
if let Some(connection) = find_connection(10) {
    println!("{}", connection.id());
}
```

В TypeScript `strictNullChecks` приближает проверку к этому, но `Option<T>` является обычным типом с методами `map`, `and_then`, `unwrap_or` и другими комбинаторами.

Не стоит повсеместно вызывать `unwrap()`. Это осознанная паника, уместная прежде всего в тестах или при доказанном инварианте.

## 10. Result вместо исключений

Ожидаемая ошибка возвращается как значение:

```rust
fn bind(address: std::net::SocketAddr) -> std::io::Result<std::net::TcpListener> {
    std::net::TcpListener::bind(address)
}
```

`Result<T, E>`:

```rust
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Оператор `?` означает: извлечь `Ok`, а `Err` немедленно вернуть, при необходимости преобразовав через `From`.

```rust
fn send_file(stream: &mut std::net::TcpStream) -> std::io::Result<()> {
    let data = std::fs::read("response.bin")?;
    use std::io::Write;
    stream.write_all(&data)?;
    Ok(())
}
```

Ошибки делятся на категории:

- ожидаемые и восстанавливаемые — `Result`;
- отсутствующие значения — `Option`;
- нарушение внутреннего инварианта — `panic!`, `assert!`;
- ошибки, которые нужно обработать локально — `match` или `if let Err(...)`.

В библиотечном коде предпочтительны конкретные error types. В приложении иногда допустим общий boxed error:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
```

В текущем проекте используется `io::Result`, потому что все ошибки первого среза относятся к I/O.

## 11. Pattern matching

`match` одновременно заменяет часть `switch`, destructuring и проверку типа:

```rust
match listener.accept() {
    Ok((stream, address)) => println!("connected: {address}"),
    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
    Err(error) => eprintln!("accept failed: {error}"),
}
```

Важное свойство — исчерпывающая проверка. После добавления нового варианта enum компилятор покажет места, где его забыли обработать.

Destructuring работает и для структур:

```rust
struct Limits {
    connections: usize,
    buffer_size: usize,
}

let Limits {
    connections,
    buffer_size,
} = limits;
```

## 12. Коллекции и итераторы

Частые соответствия:

| PHP | Python | TypeScript | Rust |
|---|---|---|---|
| `array` как список | `list` | `Array<T>` | `Vec<T>` |
| ассоциативный `array` | `dict` | `Map<K, V>` | `HashMap<K, V>` |
| `foreach` | `for` | `for...of` | `for` / `Iterator` |

Rust различает три режима итерации:

```rust
let mut names = vec![String::from("tcp"), String::from("udp")];

for name in &names {
    println!("{name}");
}

for name in &mut names {
    name.make_ascii_uppercase();
}

for name in names {
    println!("{name}");
}
```

Последний цикл потребляет `names`.

Цепочки итераторов ленивы:

```rust
let ports: Vec<u16> = [80_u16, 443, 8080]
    .into_iter()
    .filter(|port| *port >= 1024)
    .collect();
```

В отличие от многих цепочек `array_map`/`array_filter`, итераторы Rust обычно не создают промежуточные коллекции.

## 13. Замыкания и callbacks

Замыкание может захватывать окружение:

```rust
let prefix = String::from("client");
let format_id = move |id: u64| format!("{prefix}-{id}");
```

`move` перемещает захваченные значения внутрь closure. Это часто требуется при передаче работы в поток.

Есть три основных callback-trait:

- `Fn` — не изменяет и не потребляет захваченное состояние;
- `FnMut` — может изменять захваченное состояние;
- `FnOnce` — может быть вызван хотя бы один раз и вправе потребить захваченное.

Каждый `Fn` также подходит там, где требуется `FnMut` или `FnOnce`, но не наоборот.

В текущем сервере callbacks представлены trait `ServerHandler`, а не набором closure-полей. Это даёт:

- единый тип обработчика;
- методы по умолчанию;
- удобное тестирование;
- возможность хранить состояние в handler;
- отсутствие сложных типов вида `Arc<dyn Fn(...) + Send + Sync>` в публичной конфигурации.

Состояние handler при необходимости можно хранить через атомики или mutex.

## 14. Многопоточность: Send, Sync, Arc, Mutex

Rust предотвращает data race на этапе компиляции.

- `Send` — значение можно передать в другой поток;
- `Sync` — `&T` можно безопасно разделять между потоками;
- `Arc<T>` — атомарный shared ownership;
- `Mutex<T>` — эксклюзивный изменяемый доступ;
- атомики — операции над простыми числовыми состояниями без mutex.

`Rc<T>` подходит только одному потоку. Для сервера используется `Arc<T>`:

```rust
let handler = std::sync::Arc::new(handler);
let worker_handler = std::sync::Arc::clone(&handler);

std::thread::spawn(move || {
    worker_handler.on_server_start(&config);
});
```

`Arc::clone` копирует не весь handler, а увеличивает счётчик владельцев.

Изменяемое разделяемое состояние:

```rust
use std::sync::{Arc, Mutex};

let messages = Arc::new(Mutex::new(Vec::<String>::new()));
let worker_messages = Arc::clone(&messages);

std::thread::spawn(move || {
    worker_messages.lock().unwrap().push(String::from("hello"));
});
```

Mutex guard освобождает блокировку через `Drop`.

Для счётчиков сервер использует `AtomicU64`. `Ordering` описывает гарантии видимости между потоками:

- `Relaxed` подходит независимой статистике;
- `Acquire`/`Release` используются для согласования флага остановки;
- `SeqCst` даёт наиболее строгую модель, но не должен быть автоматическим выбором вместо понимания протокола синхронизации.

### 14.1 Текущая потоковая модель

Сейчас сервер создаёт один OS thread на TCP-соединение. Плюсы:

- понятный blocking I/O;
- только стандартная библиотека;
- удобное изучение ownership и `Send`/`Sync`;
- простая трассировка выполнения.

Ограничения:

- поток имеет заметную стоимость памяти и переключения контекста;
- число одновременно открытых соединений необходимо ограничивать;
- для десятков тысяч соединений нужен event loop или async runtime.

Это осознанный учебный этап, а не финальная high-load архитектура.

## 15. Async — не «автоматически быстрее»

`async fn` возвращает `Future`. Она выполняется только когда runtime или другой executor опрашивает future.

```rust
async fn load() -> std::io::Result<Vec<u8>> {
    todo!()
}
```

Стандартная библиотека определяет `Future`, но не предоставляет полноценный сетевой async runtime. Обычно используют Tokio или async-std, однако добавлять их до понимания blocking-версии не обязательно.

Async полезен, когда много задач преимущественно ждут I/O. Для CPU-bound работы всё равно нужны потоки или отдельный пул.

Переход сервера к async изменит внутренний механизм ожидания, но публичные понятия `ServerConfig`, `Connection` и `ServerHandler` желательно сохранить или адаптировать минимально.

## 16. Почему нельзя буквально транслировать PHP

Механический перенос приводит к архитектуре, которая борется с Rust.

### Глобальные static-состояния

PHP-сервер хранит множество глобальных mutable static-полей. В Rust глобальная изменяемость требует синхронизации и усложняет тесты. Лучше, чтобы состояние принадлежало `Server`:

```rust
pub struct Server<H> {
    config: ServerConfig,
    handler: std::sync::Arc<H>,
    statistics: std::sync::Arc<ConnectionStatistics>,
}
```

### Динамические свойства

PHP `stdClass`, mixed и динамические поля позволяют прикрепить любые данные. В Rust состояние лучше моделировать конкретным типом или generic context. Это требует больше проектирования, но исключает опечатки и неверные типы в runtime.

### Наследование callbacks

PHP `ServerAbstract` может дать пустые методы наследнику. В Rust тот же смысл естественно выражается trait-методами по умолчанию:

```rust
pub trait ServerHandler {
    fn on_connect(&self, _connection: &mut dyn Connection) {}
}
```

### Исключения из глубины стека

В Rust ошибка обычно проходит через сигнатуру `Result`. Видно, какие функции могут завершиться ошибкой, а `?` оставляет распространение компактным.

### Один универсальный array

PHP array одновременно является списком и ordered map. Rust заставляет выбрать `Vec`, `HashMap`, `BTreeMap`, массив фиксированной длины или slice. Выбор структуры данных становится частью контракта.

## 17. Как читать текущий server-rs

Рекомендуемый порядок:

1. `src/main.rs` — создание конфигурации и реализация echo-handler.
2. `src/server_abstract.rs` — события и значения по умолчанию.
3. `src/connection/base.rs` — контракт соединения и статистика.
4. `src/connection/tcp_connection.rs` — владение сокетом, чтение, запись и `Drop`.
5. `src/server.rs` — bind, accept loop, потоки и остановка.
6. `tests/tcp_server.rs` — использование публичного API снаружи crate.

Поток данных:

```text
TcpListener::accept
        |
        v
TcpConnection::new
        |
        v
ServerHandler::on_connect
        |
        v
TcpConnection::read -> ServerHandler::on_message -> Connection::send
        |
        v
ServerHandler::on_close -> Drop
```

Обрати внимание на границы владения:

- `Server` владеет конфигурацией;
- `Arc` совместно владеет handler и статистикой;
- worker thread владеет конкретным `TcpConnection`;
- callback только временно получает `&mut dyn Connection`;
- входной пакет передаётся как `&[u8]` без копирования.

## 18. Инструменты ежедневной разработки

```text
cargo check
cargo test
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --open
```

- `cargo check` быстро проверяет типы без полноценной сборки бинарника;
- `cargo test` запускает unit-, integration- и doc-тесты;
- `cargo fmt` приводит код к единому стандартному стилю;
- `cargo clippy` находит неидиоматичные и подозрительные конструкции;
- `cargo doc` строит документацию по публичному API.

Компилятор Rust стоит читать сверху вниз. Первая ошибка часто вызывает каскад последующих. Полезный порядок:

1. понять, кто должен владеть значением;
2. проверить, должен ли параметр быть `T`, `&T` или `&mut T`;
3. уменьшить время жизни заимствования;
4. только затем рассматривать `clone`, `Arc`, `RefCell` или `Mutex`.

## 19. Тестирование

Unit-тесты обычно находятся рядом с кодом:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn adds_values() {
        assert_eq!(2 + 2, 4);
    }
}
```

Интеграционные тесты в `tests/` видят crate как внешний пользователь. Поэтому они проверяют не приватные детали, а реальный публичный API.

Текущий TCP-тест:

1. получает свободный локальный порт;
2. запускает сервер в отдельном потоке;
3. подключает реальный `TcpStream`;
4. отправляет байты и проверяет echo;
5. останавливает сервер через `ServerHandle`;
6. проверяет финальную статистику.

Это надёжнее теста, который просто вызывает внутренние методы и никогда не затрагивает настоящий сокет.

## 20. Частые ошибки после PHP, Python и TypeScript

### Использовать `String` в каждом параметре

Если функция только читает текст, обычно принимай `&str`. Для байтов — `&[u8]`.

### Безусловно вызывать `clone()`

Сначала выясни владельца. Возможно, функция должна принимать ссылку или забирать значение.

### Заворачивать всё в `Arc<Mutex<_>>`

Это аналог «сделать всё глобальным объектом». Сначала раздели владение по компонентам; синхронизацию добавляй только для действительно общего mutable state.

### Пытаться создать иерархию классов

Разделяй данные на структуры, поведение на traits, а повторное использование строй композицией.

### Превращать каждую ошибку в panic

`unwrap()` не является аналогом `?`. `unwrap()` завершает поток при ошибке, `?` возвращает ошибку вызывающему коду.

### Хранить ссылки в долгоживущих структурах слишком рано

Это приводит к сложным lifetime-параметрам. Для конфигурации и runtime-состояния часто проще владеть данными.

### Сразу переходить к unsafe

`unsafe` не отключает borrow checker целиком, но переносит часть доказательств безопасности на разработчика. Для этого проекта на текущих этапах он не нужен.

### Писать собственный event loop до работающего протокола

Сначала полезнее реализовать корректное фреймирование, лимиты пакетов, backpressure и тесты. Иначе одновременно отлаживаются слишком многие новые концепции.

## 21. План обучения на этом проекте

### Этап 1. Базовый TCP — выполнен

Темы:

- package, crate и modules;
- structs и traits;
- ownership сокета;
- `Result` и `io::Error`;
- потоки, `Arc`, атомики;
- integration tests.

Самостоятельные упражнения:

1. Добавить `max_connections` в `ServerConfig`.
2. Добавить в статистику общее число принятых соединений, не уменьшаемое при закрытии.
3. Реализовать `Display` для `ServerStatus`.
4. Проверить callback `on_connect` отдельным тестом.

### Этап 2. Протоколы Text и Frame

Темы:

- trait `Protocol`;
- associated types;
- slices и работа с байтами;
- неполные TCP-пакеты;
- `Vec<u8>` и буферизация;
- проверка максимального размера сообщения.

Важно понять: один `read` не равен одному сообщению. TCP — поток байтов. Одно сообщение может прийти частями, а несколько сообщений — одним чтением.

Для Frame-протокола можно использовать заголовок длины:

```text
[4 bytes big-endian length][payload]
```

### Этап 3. HTTP request/response

Темы:

- enums для методов и версий;
- borrowing частей входного буфера;
- `HashMap` или специализированное хранение headers;
- builder API для response;
- разделение parsing и transport;
- fuzz-friendly parser без panic на пользовательском вводе.

Не нужно начинать с multipart, cookies и sessions. Сначала достаточно request line, headers и `Content-Length`.

### Этап 4. Event loop и backpressure

Темы:

- nonblocking sockets;
- readiness;
- state machines;
- очередь записи;
- high/low water marks;
- ограничение ресурсов.

Здесь станет видно, стоит ли писать учебный event loop на системных API или подключить Tokio для прикладной версии.

### Этап 5. WebSocket

Темы:

- HTTP Upgrade;
- бинарный формат frame;
- masking;
- continuation frames;
- ping/pong и close handshake;
- защита от слишком больших сообщений.

### Этап 6. Управление процессом

Темы:

- сигналы на Unix;
- graceful shutdown;
- platform-specific modules через `#[cfg(unix)]` и `#[cfg(windows)]`;
- PID/status files;
- structured logging.

Процессную модель исходного PHP-сервера не следует переносить первой: она платформенно-зависима и отвлекает от сетевого ядра.

## 22. Архитектурные ориентиры для дальнейшего переноса

Сохранять стоит не буквальную форму PHP-классов, а проверенные доменные границы:

- Server — lifecycle и приём соединений;
- Connection — транспорт и буферы;
- Protocol — границы сообщений и кодирование;
- Handler — пользовательские события;
- Event loop — ожидание готовности I/O;
- Timer — планирование по времени.

Желательные зависимости:

```text
application handler
        |
        v
protocol abstraction
        |
        v
connection abstraction
        |
        v
event loop / operating system
```

Protocol не должен управлять процессами, Connection не должен разбирать HTTP, а event loop не должен знать о WebSocket. Это позволит менять blocking transport на async без переписывания протоколов целиком.

## 23. Краткая шпаргалка соответствий

| Задача | PHP/Python/TS | Rust |
|---|---|---|
| Необязательное значение | `null`, `None`, `undefined` | `Option<T>` |
| Успех или ошибка | исключение | `Result<T, E>` |
| Объект с полями | class/object | `struct` |
| Контракт поведения | interface/protocol | `trait` |
| Закрытая иерархия вариантов | enum/union | `enum` |
| Динамический интерфейс | object/interface reference | `&dyn Trait`, `Box<dyn Trait>` |
| Общий владелец в одном потоке | обычная ссылка | `Rc<T>` |
| Общий владелец между потоками | обычная ссылка/runtime | `Arc<T>` |
| Синхронизация изменений | lock/runtime conventions | `Mutex<T>`, `RwLock<T>`, atomics |
| Список | array/list/Array | `Vec<T>` |
| Словарь | array/dict/Map | `HashMap<K, V>` |
| Деструктор/ finally | `__destruct`, context manager | `Drop`/RAII |
| Callback | callable/function | closure или trait |
| Проверка типа в runtime | `instanceof`, `isinstance` | generics, traits, `enum` |

## 24. Как эффективно осваивать Rust

Не пытайся запомнить все lifetime-аннотации заранее. Для практики важнее каждый раз отвечать на четыре вопроса:

1. Кто владеет этим значением?
2. Кто только временно его читает?
3. Кто и когда имеет право его менять?
4. Может ли значение пережить поток, callback или async-задачу, куда оно передаётся?

Если ответы ясны, типы Rust обычно получаются естественно. Если приходится беспорядочно добавлять `clone`, `Arc<Mutex<_>>`, `'static` или `unsafe`, чаще всего не определена граница владения или компонент выполняет слишком много обязанностей.

Лучший учебный цикл для этого проекта:

1. взять маленькую возможность исходного PHP-сервера;
2. сформулировать её контракт без привязки к PHP;
3. определить владельцев данных;
4. написать минимальный Rust API;
5. сначала написать проверяемый сценарий;
6. реализовать его на стандартной библиотеке;
7. прогнать `fmt`, тесты и Clippy;
8. только затем обобщать архитектуру.

Так переписывание становится не переводом синтаксиса, а последовательным изучением системного проектирования на Rust.
