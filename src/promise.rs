// Copyright 2015 The coio Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Promise style asynchronous APIs

use std::fmt::Debug;
use scheduler::{JoinHandle, Scheduler};
use options::Options;

/// Store the result of an coroutine, return Ok(T) if succeeded, Err(E) otherwise.
pub struct Promise<'scope, T, E>
    where T: Send + 'scope,
          E: Send + Debug + 'scope
{
    join_handle: JoinHandle<'scope, Result<T, E>>,
}

impl<'scope, T, E> Promise<'scope, T, E>
    where T: Send + 'scope,
          E: Send + Debug + 'scope
{
    /// Spawn a new coroutine to execute the task
    pub fn spawn<F>(f: F) -> Self
        where F: FnOnce() -> Result<T, E> + Send + 'scope
    {
        Promise { join_handle: Scheduler::spawn(f) }
    }

    /// Spawn a new coroutine with options to execute the task
    pub fn spawn_opts<F>(f: F, opts: Options) -> Self
        where F: FnOnce() -> Result<T, E> + Send + 'scope
    {
        Promise { join_handle: Scheduler::spawn_opts(f, opts) }
    }

    /// Synchronize the execution with the caller and retrive the result
    pub fn sync(self) -> Result<T, E> {
        self.join_handle.join().unwrap()
    }

    /// Execute one of the function depending on the result of the current coroutine
    pub fn then<TT, EE, FT, FE>(self, ft: FT, fe: FE) -> Promise<'scope, TT, EE>
        where TT: Send + 'scope,
              EE: Send + Debug + 'scope,
              FT: Send + 'scope + FnOnce(T) -> Result<TT, EE>,
              FE: Send + 'scope + FnOnce(E) -> Result<TT, EE>
    {
        let join_handle = Scheduler::spawn(move || {
            match self.join_handle.join().unwrap() {
                Ok(t) => ft(t),
                Err(e) => fe(e),
            }
        });

        Promise { join_handle: join_handle }
    }

    /// Run the function with the result of the current task
    pub fn chain<TT, EE, F>(self, f: F) -> Promise<'scope, TT, EE>
        where TT: Send + 'scope,
              EE: Send + Debug + 'scope,
              F: Send + 'scope + FnOnce(Result<T, E>) -> Result<TT, EE>
    {
        let join_handle = Scheduler::spawn(move || f(self.join_handle.join().unwrap()));

        Promise { join_handle: join_handle }
    }

    /// Execute the function of the result is `Ok`, otherwise it will just return the value
    pub fn success<TT, F>(self, f: F) -> Promise<'scope, TT, E>
        where TT: Send + 'scope,
              F: FnOnce(T) -> Result<TT, E> + Send + 'scope
    {
        let join_handle = Scheduler::spawn(move || {
            match self.join_handle.join().unwrap() {
                Ok(t) => f(t),
                Err(e) => Err(e),
            }
        });

        Promise { join_handle: join_handle }
    }

    /// Execute the function of the result is `Err`, otherwise it will just return the value
    pub fn fail<F>(self, f: F) -> Promise<'scope, T, E>
        where F: Send + 'scope + FnOnce(E) -> Result<T, E>
    {
        let join_handle = Scheduler::spawn(move || {
            match self.join_handle.join().unwrap() {
                Ok(t) => Ok(t),
                Err(e) => f(e),
            }
        });

        Promise { join_handle: join_handle }
    }

    /// Execute the function with the result of the previous promise asynchronously
    pub fn finally<F>(self, f: F)
        where F: Send + 'scope + FnOnce(Result<T, E>)
    {
        Scheduler::spawn(move || f(self.join_handle.join().unwrap()));
    }

    /// Execute the function with the result of the previous promise synchronously
    pub fn finally_sync<F>(self, f: F)
        where F: Send + 'scope + FnOnce(Result<T, E>)
    {
        f(self.sync())
    }
}

