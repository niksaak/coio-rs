// Copyright 2015 The coio Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Coroutine scheduling with asynchronous I/O support

#![feature(
    arc_counts,
    asm,
    fnbox,
    optin_builtin_traits,
    reflect_marker,
    shared,
)]

#[macro_use]
extern crate log;

extern crate context;
extern crate deque;
extern crate libc;
extern crate linked_hash_map;
extern crate mio;
extern crate rand;
extern crate slab;
extern crate time;

#[cfg(test)]
extern crate env_logger;

pub mod join_handle;
pub mod net;
pub mod options;
pub mod promise;
pub mod scheduler;
pub mod sync;

pub use options::Options;
pub use promise::Promise;
pub use scheduler::{Scheduler, JoinHandle};

mod coroutine;
mod runtime;

use std::thread;
use std::time::Duration;

#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

/// Spawn a new Coroutine
#[inline]
pub fn spawn<'scope, F, T>(f: F) -> JoinHandle<'scope, T>
    where F: FnOnce() -> T + Send + 'scope,
          T: Send + 'scope
{
    Scheduler::spawn(f)
}

/// Spawn a new Coroutine with options
#[inline]
pub fn spawn_opts<'scope, F, T>(f: F, opts: Options) -> JoinHandle<'scope, T>
    where F: FnOnce() -> T + Send + 'scope,
          T: Send + 'scope
{
    Scheduler::spawn_opts(f, opts)
}

/// Give up the CPU
#[inline]
pub fn sched() {
    Scheduler::sched()
}

/// Put the current coroutine to sleep for the specific amount of time
#[inline]
pub fn sleep_ms(ms: u64) {
    sleep(Duration::from_millis(ms))
}

/// Put the current coroutine to sleep for the specific amount of time
#[inline]
pub fn sleep(dur: Duration) {
    match Scheduler::instance() {
        Some(s) => s.sleep(dur),
        None => thread::sleep(dur),
    }
}

/// Coroutine configuration. Provides detailed control over
/// the properties and behavior of new coroutines.
pub struct Builder {
    opts: Options,
}

impl Builder {
    /// Generates the base configuration for spawning a coroutine,
    // from which configuration methods can be chained.
    #[inline]
    pub fn new() -> Builder {
        Builder { opts: Options::new() }
    }

    /// Sets the size of the stack for the new coroutine.
    #[inline]
    pub fn stack_size(mut self, stack_size: usize) -> Builder {
        self.opts.stack_size = stack_size;
        self
    }

    /// Names the coroutine-to-be. Currently the name
    // is used for identification only in panic messages.
    #[inline]
    pub fn name(mut self, name: String) -> Builder {
        self.opts.name = Some(name);
        self
    }

    /// Spawn a new coroutine
    #[inline]
    pub fn spawn<'scope, F, T>(self, f: F) -> JoinHandle<'scope, T>
        where F: FnOnce() -> T + Send + 'scope,
              T: Send + 'scope
    {
        Scheduler::spawn_opts(f, self.opts)
    }
}


#[cfg(debug_assertions)]
static GLOBAL_WORK_COUNT: AtomicUsize = ATOMIC_USIZE_INIT;

#[inline]
#[cfg(debug_assertions)]
fn global_work_count_add() {
    GLOBAL_WORK_COUNT.fetch_add(1, Ordering::Relaxed);
}

#[inline]
#[cfg(debug_assertions)]
fn global_work_count_sub() {
    GLOBAL_WORK_COUNT.fetch_sub(1, Ordering::Relaxed);
}

#[inline]
#[cfg(debug_assertions)]
fn global_work_count_get() -> usize {
    GLOBAL_WORK_COUNT.load(Ordering::Relaxed)
}

#[inline]
#[cfg(not(debug_assertions))]
fn global_work_count_add() {}

#[inline]
#[cfg(not(debug_assertions))]
fn global_work_count_get() -> usize {
    0
}

fn duration_to_ms(dur: Duration) -> u64 {
    dur.as_secs() * 1_000 + dur.subsec_nanos() as u64 / 1_000_000
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sleep_ms() {
        Scheduler::new()
            .run(|| {
                sleep_ms(1000);
            })
            .unwrap();
    }
}
