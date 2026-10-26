//! Promises and `async` blocks, as Kotlin Temper's interpreter runs them
//! (`Promises.kt`, `AsyncFn.kt`). `async { … }` makes its block's generator
//! and queues it; queued blocks run after the code that's running, a
//! module's top level, ends. A block runs until it ends or pauses at an
//! `await` of a promise that isn't resolved yet; resolving the promise
//! queues every block awaiting it, in the order they began to wait.

use super::{Abrupt, Generator, Interp, R, Val, fail, type_name};
use crate::settle::Settled;
use std::cell::RefCell;
use std::rc::Rc;

/// A promise, shared by its `PromiseBuilder` and whoever awaits it.
#[derive(Debug, Default)]
pub struct Promise {
    state: RefCell<State>,
}

#[derive(Debug)]
enum State {
    /// Not resolved yet, with the `async` blocks paused awaiting it.
    Pending(Vec<Rc<Generator>>),
    Complete(Val),
    Broken,
}

impl Default for State {
    fn default() -> State {
        State::Pending(Vec::new())
    }
}

impl Promise {
    /// What `await` gives, once the promise is resolved: its value, or
    /// for a broken one, a failure.
    pub(super) fn outcome(&self) -> Option<R> {
        match &*self.state.borrow() {
            State::Pending(_) => None,
            State::Complete(value) => Some(Ok(value.clone())),
            State::Broken => Some(Err(Abrupt::Fail(None))),
        }
    }
}

impl Interp {
    /// `async { … }`: makes the block's generator, which runs only through
    /// its parameters, and queues it.
    pub(super) fn start_async(&mut self, s: &Settled, args: Vec<Val>) -> R {
        let [block] = <[Val; 1]>::try_from(args).map_err(|_| fail("async takes a generator block"))?;
        match self.apply(s, block, Vec::new())? {
            Val::Generator(generator) => {
                self.ready.push_back(generator);
                Ok(Val::Void)
            }
            other => Err(fail(format!("Expected a SafeGenerator, not {}", type_name(&other)))),
        }
    }

    /// Runs queued `async` blocks until none are ready. A block that
    /// fails doesn't stop the others; the first failure is the result. A
    /// `yield` pauses a block for good, as Kotlin Temper's stepper ignores
    /// what `next()` gives.
    pub(super) fn run_tasks(&mut self, s: &Settled) -> Result<(), Abrupt> {
        let mut result = Ok(());
        while let Some(task) = self.ready.pop_front() {
            match self.step(s, &task) {
                Err(Abrupt::Await(promise)) => match &mut *promise.state.borrow_mut() {
                    State::Pending(waiters) => waiters.push(task),
                    _ => unreachable!("a block pauses only on a pending promise"),
                },
                Ok(_) | Err(Abrupt::Yield(_)) => {}
                Err(abrupt) => result = result.and(Err(abrupt)),
            }
        }
        result
    }

    /// `PromiseBuilder`'s methods. The first resolution wins; later ones
    /// do nothing, as in Kotlin Temper.
    pub(super) fn promise_builder_method(&mut self, promise: &Promise, name: &str, args: &[Val]) -> Option<R> {
        let resolution = match (name, args) {
            ("complete", [value]) => State::Complete(value.clone()),
            ("breakPromise", []) => State::Broken,
            _ => return None,
        };
        let mut state = promise.state.borrow_mut();
        if let State::Pending(waiters) = &mut *state {
            self.ready.extend(waiters.drain(..));
            *state = resolution;
        }
        Some(Ok(Val::Void))
    }
}
