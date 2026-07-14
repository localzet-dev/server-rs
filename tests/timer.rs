use std::sync::mpsc;
use std::time::Duration;

use server_rs::Timer;

#[test]
fn delayed_timer_runs_once() {
    let timer = Timer::new();
    let (sender, receiver) = mpsc::channel();
    timer.delay(Duration::from_millis(5), move || sender.send(1).unwrap());

    assert_eq!(receiver.recv_timeout(Duration::from_secs(1)).unwrap(), 1);
    assert!(receiver.recv_timeout(Duration::from_millis(20)).is_err());
}

#[test]
fn repeating_timer_can_cancel_itself_from_another_thread() {
    let timer = Timer::new();
    let (sender, receiver) = mpsc::channel();
    let id = timer
        .repeat(Duration::from_millis(5), move || sender.send(()).unwrap())
        .unwrap();

    receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(!timer.delete(id) || timer.pending_count() == 0);
    assert!(receiver.recv_timeout(Duration::from_millis(30)).is_err());
}

#[test]
fn deleting_delayed_timer_prevents_callback() {
    let timer = Timer::new();
    let (sender, receiver) = mpsc::channel();
    let id = timer.delay(Duration::from_millis(50), move || sender.send(()).unwrap());

    assert!(timer.delete(id));
    assert!(receiver.recv_timeout(Duration::from_millis(80)).is_err());
}
