use std::collections::{BTreeMap, HashSet};
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

type Callback = Box<dyn FnMut() + Send + 'static>;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TimerId(u64);

impl TimerId {
    pub fn get(self) -> u64 {
        self.0
    }
}

struct Task {
    id: TimerId,
    interval: Option<Duration>,
    callback: Callback,
}

#[derive(Default)]
struct State {
    tasks: BTreeMap<(Instant, TimerId), Task>,
    cancelled: HashSet<TimerId>,
    stopped: bool,
}

#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

pub struct Timer {
    shared: Arc<Shared>,
    next_id: AtomicU64,
    worker: Option<JoinHandle<()>>,
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

impl Timer {
    pub fn new() -> Self {
        let shared = Arc::new(Shared::default());
        let worker_shared = Arc::clone(&shared);
        let worker = thread::spawn(move || run_scheduler(worker_shared));
        Self {
            shared,
            next_id: AtomicU64::new(1),
            worker: Some(worker),
        }
    }

    pub fn delay<F>(&self, delay: Duration, callback: F) -> TimerId
    where
        F: FnOnce() + Send + 'static,
    {
        let mut callback = Some(callback);
        self.add(
            delay,
            None,
            Box::new(move || {
                if let Some(callback) = callback.take() {
                    callback();
                }
            }),
        )
    }

    pub fn repeat<F>(&self, interval: Duration, callback: F) -> io::Result<TimerId>
    where
        F: FnMut() + Send + 'static,
    {
        if interval.is_zero() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "repeating timer interval must be greater than zero",
            ));
        }
        Ok(self.add(interval, Some(interval), Box::new(callback)))
    }

    pub fn delete(&self, id: TimerId) -> bool {
        let mut state = lock_state(&self.shared);
        let before = state.tasks.len();
        state.tasks.retain(|(_, task_id), _| *task_id != id);
        let removed = state.tasks.len() != before;
        state.cancelled.insert(id);
        drop(state);
        self.shared.changed.notify_one();
        removed
    }

    pub fn delete_all(&self) {
        let mut state = lock_state(&self.shared);
        let ids: Vec<_> = state.tasks.values().map(|task| task.id).collect();
        state.cancelled.extend(ids);
        state.tasks.clear();
        drop(state);
        self.shared.changed.notify_one();
    }

    pub fn pending_count(&self) -> usize {
        lock_state(&self.shared).tasks.len()
    }

    pub fn sleep(delay: Duration) {
        thread::sleep(delay);
    }

    fn add(&self, delay: Duration, interval: Option<Duration>, callback: Callback) -> TimerId {
        let id = TimerId(self.next_id.fetch_add(1, Ordering::Relaxed));
        let deadline = Instant::now()
            .checked_add(delay)
            .unwrap_or_else(Instant::now);
        let task = Task {
            id,
            interval,
            callback,
        };
        lock_state(&self.shared).tasks.insert((deadline, id), task);
        self.shared.changed.notify_one();
        id
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        {
            let mut state = lock_state(&self.shared);
            state.stopped = true;
            state.tasks.clear();
        }
        self.shared.changed.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_scheduler(shared: Arc<Shared>) {
    loop {
        let mut state = lock_state(&shared);
        while state.tasks.is_empty() && !state.stopped {
            state = shared
                .changed
                .wait(state)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
        }
        if state.stopped {
            return;
        }

        let (&(deadline, id), _) = state.tasks.first_key_value().expect("tasks are not empty");
        let now = Instant::now();
        if deadline > now {
            let timeout = deadline.duration_since(now);
            let (new_state, _) = shared
                .changed
                .wait_timeout(state, timeout)
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            drop(new_state);
            continue;
        }

        let mut task = state
            .tasks
            .remove(&(deadline, id))
            .expect("selected timer must exist");
        if state.cancelled.remove(&id) {
            continue;
        }
        drop(state);

        let _ = panic::catch_unwind(AssertUnwindSafe(|| (task.callback)()));

        if let Some(interval) = task.interval {
            let mut state = lock_state(&shared);
            if !state.stopped && !state.cancelled.remove(&id) {
                let next_deadline = Instant::now()
                    .checked_add(interval)
                    .unwrap_or_else(Instant::now);
                state.tasks.insert((next_deadline, id), task);
            }
        }
    }
}

fn lock_state(shared: &Shared) -> MutexGuard<'_, State> {
    shared
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
