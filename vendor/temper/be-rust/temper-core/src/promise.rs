use super::{Error, Result};
use crate::SafeGenerator;
use std::{
    collections::VecDeque,
    future::Future,
    ops::Deref,
    pin::Pin,
    sync::{Arc, Condvar, Mutex, RwLock},
    task::{Context, Poll, Waker},
};

pub trait AsyncRunnerTrait: Send + Sync {
    fn run_all_blocking(&self);

    fn run_async(&self, task: Task);
}

#[derive(Clone)]
pub struct AsyncRunner(Arc<dyn AsyncRunnerTrait>);

impl AsyncRunner {
    pub fn new(selfish: impl AsyncRunnerTrait + 'static) -> Self {
        Self(Arc::new(selfish))
    }

    pub fn run_async<T>(&self, gen: std::sync::Arc<dyn Fn() -> SafeGenerator<T> + Send + Sync>)
    where
        T: Clone + Send + Sync + 'static,
    {
        let gen = gen();
        // TODO Any way to avoid the extra Arc wrapping?
        let gen_ignoring_result = Arc::new(move || {
            let _ = gen.next_safe();
        });
        self.0.run_async(gen_ignoring_result);
    }
}

impl Deref for AsyncRunner {
    type Target = dyn AsyncRunnerTrait;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

struct SingleThreadAsyncRunnerStruct {
    tasks: VecDeque<Task>,
}

pub struct SingleThreadAsyncRunner(Arc<RwLock<SingleThreadAsyncRunnerStruct>>);

impl SingleThreadAsyncRunner {
    pub fn new() -> AsyncRunner {
        AsyncRunner::new(SingleThreadAsyncRunner(Arc::new(RwLock::new(
            SingleThreadAsyncRunnerStruct {
                tasks: VecDeque::new(),
            },
        ))))
    }
}

impl AsyncRunnerTrait for SingleThreadAsyncRunner {
    fn run_all_blocking(&self) {
        loop {
            let task = {
                let mut lock = self.0.write().unwrap();
                lock.tasks.pop_front()
            };
            let Some(task) = task else {
                break;
            };
            task();
        }
    }

    fn run_async(&self, task: Task) {
        let mut lock = self.0.write().unwrap();
        lock.tasks.push_back(task);
    }
}

#[derive(Clone)]
pub struct Promise<T>
where
    T: Clone,
{
    // The result and the waiters share one mutex, so a waiter added while
    // another thread resolves the promise is either kept or sees the result.
    state: StatePair<T>,
}

impl<T> Promise<T>
where
    T: Clone,
{
    pub fn get(&self) -> Result<T> {
        let (lock, cvar) = &*self.state;
        let mut state = lock.lock().unwrap();
        while state.result.is_none() {
            state = cvar.wait(state).unwrap();
        }
        state.result.clone().unwrap()
    }

    /// Runs `next` once this promise is resolved: now if it already is,
    /// otherwise inside the call that resolves it, after the waiters added
    /// before it.
    pub fn on_ready(&self, next: Task) {
        {
            let mut state = self.state.0.lock().unwrap();
            if state.result.is_none() {
                state.waiters.push(next);
                return;
            }
        }
        next();
    }
}

/// A promise is a `Future` of its result, so Rust code can `.await` it on
/// any executor. It adds no bounds of its own: it's `Send` exactly when
/// its value type is, and `Unpin`.
impl<T> Future for Promise<T>
where
    T: Clone,
{
    type Output = Result<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<T>> {
        let mut state = self.state.0.lock().unwrap();
        if let Some(result) = &state.result {
            return Poll::Ready(result.clone());
        }
        // One waker per task: a later poll's replaces its earlier one's.
        let waker = cx.waker();
        if !state.wakers.iter().any(|w| w.will_wake(waker)) {
            state.wakers.push(waker.clone());
        }
        Poll::Pending
    }
}

#[derive(Clone)]
pub struct PromiseBuilder<T>
where
    T: Clone,
{
    promise: Promise<T>,
}

impl<T> PromiseBuilder<T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Self {
            promise: Promise {
                state: Arc::new((
                    Mutex::new(PromiseState {
                        result: None,
                        waiters: Vec::new(),
                        wakers: Vec::new(),
                    }),
                    Condvar::new(),
                )),
            },
        }
    }

    pub fn break_promise(&self) {
        self.resolve(Err(Error::new()));
    }

    pub fn complete(&self, value: T) {
        self.resolve(Ok(value));
    }

    pub fn promise(&self) -> Promise<T> {
        self.promise.clone()
    }

    /// The first resolution sticks. Later ones change nothing and wake no one.
    fn resolve(&self, result: Result<T>) {
        let (lock, cvar) = &*self.promise.state;
        let (waiters, wakers) = {
            let mut state = lock.lock().unwrap();
            if state.result.is_some() {
                return;
            }
            state.result = Some(result);
            (
                std::mem::take(&mut state.waiters),
                std::mem::take(&mut state.wakers),
            )
        };
        cvar.notify_all();
        for waker in wakers {
            waker.wake();
        }
        // Outside the lock, so a waiter may await this promise again.
        for next in waiters {
            next();
        }
    }
}

struct PromiseState<T> {
    result: Option<Result<T>>,
    waiters: Vec<Task>,
    /// The tasks of Rust code that `.await`s it ([`Future`]).
    wakers: Vec<Waker>,
}

type StatePair<T> = Arc<(Mutex<PromiseState<T>>, Condvar)>;

pub type Task = std::sync::Arc<dyn Fn() + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::*;

    fn log_task(log: &Arc<Mutex<Vec<String>>>, line: &str) -> Task {
        let log = log.clone();
        let line = line.to_string();
        Arc::new(move || log.lock().unwrap().push(line.clone()))
    }

    fn lines(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
        log.lock().unwrap().clone()
    }

    #[test]
    fn every_waiter_resumes_on_complete() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let pb = PromiseBuilder::<i32>::new();
        pb.promise().on_ready(log_task(&log, "first"));
        pb.promise().on_ready(log_task(&log, "second"));
        pb.complete(7);
        assert_eq!(lines(&log), ["first", "second"]);
        assert_eq!(pb.promise().get().ok(), Some(7));
    }

    #[test]
    fn every_waiter_resumes_on_break_promise() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let pb = PromiseBuilder::<i32>::new();
        pb.promise().on_ready(log_task(&log, "first"));
        pb.promise().on_ready(log_task(&log, "second"));
        pb.break_promise();
        assert_eq!(lines(&log), ["first", "second"]);
        assert!(pb.promise().get().is_err());
    }

    #[test]
    fn on_ready_after_resolution_runs_now() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let pb = PromiseBuilder::<i32>::new();
        pb.complete(1);
        pb.promise().on_ready(log_task(&log, "late"));
        assert_eq!(lines(&log), ["late"]);
    }

    #[test]
    fn first_resolution_sticks_and_wakes_waiters_once() {
        let log = Arc::new(Mutex::new(Vec::new()));
        let pb = PromiseBuilder::<i32>::new();
        pb.promise().on_ready(log_task(&log, "woken"));
        pb.complete(1);
        pb.complete(2);
        pb.break_promise();
        assert_eq!(lines(&log), ["woken"]);
        assert_eq!(pb.promise().get().ok(), Some(1));
    }

    #[test]
    fn get_blocks_until_another_thread_completes() {
        let pb = PromiseBuilder::<i32>::new();
        let promise = pb.promise();
        let waiter = std::thread::spawn(move || promise.get().ok());
        std::thread::sleep(std::time::Duration::from_millis(20));
        pb.complete(5);
        assert_eq!(waiter.join().unwrap(), Some(5));
    }

    /// Runs a future to its end on this thread, with nothing but std.
    fn block_on<F: Future>(future: F) -> F::Output {
        struct Unpark(std::thread::Thread);
        impl std::task::Wake for Unpark {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(Unpark(std::thread::current())));
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(future);
        loop {
            if let Poll::Ready(output) = future.as_mut().poll(&mut cx) {
                return output;
            }
            std::thread::park();
        }
    }

    fn assert_send_static<F: Future + Send + 'static>(_: &F) {}

    #[test]
    fn a_promise_is_a_future_that_any_executor_may_await() {
        let pb = PromiseBuilder::<i32>::new();
        let promise = pb.promise();
        assert_send_static(&promise);
        let awaited = async move { promise.await.ok() };
        assert_send_static(&awaited);
        let completer = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(20));
            pb.complete(5);
        });
        assert_eq!(block_on(awaited), Some(5));
        completer.join().unwrap();
        let broken = PromiseBuilder::<i32>::new();
        broken.break_promise();
        assert!(block_on(broken.promise()).is_err());
    }

    #[test]
    fn a_promise_of_a_value_that_is_not_send_is_awaitable_on_this_thread() {
        let pb = PromiseBuilder::<std::rc::Rc<i32>>::new();
        let promise = pb.promise();
        pb.complete(std::rc::Rc::new(3));
        assert_eq!(block_on(async { *promise.await.ok().unwrap() }), 3);
    }

    #[test]
    fn promise_and_builder_are_send_and_sync() {
        fn check<X: Send + Sync>() {}
        check::<Promise<i32>>();
        check::<PromiseBuilder<i32>>();
    }
}
