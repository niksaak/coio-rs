// Copyright 2015 The coio Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::sync::Arc;
use std::thread;

use sync::mono_barrier::MonoBarrier;

struct JoinHandleInner<'scope, T: 'scope> {
    barrier: MonoBarrier,
    data: UnsafeCell<Option<thread::Result<T>>>,
    scope: PhantomData<&'scope T>,
}

unsafe impl<'scope, T: Send> Send for JoinHandleInner<'scope, T> {}
unsafe impl<'scope, T> Sync for JoinHandleInner<'scope, T> {}

impl<'scope, T> JoinHandleInner<'scope, T> {
    fn new() -> JoinHandleInner<'scope, T> {
        JoinHandleInner {
            barrier: MonoBarrier::new(),
            data: UnsafeCell::new(None),
            scope: PhantomData,
        }
    }
}

pub struct JoinHandleSender<'scope, T: 'scope> {
    inner: Arc<JoinHandleInner<'scope, T>>,
}

impl<'scope, T: 'scope> JoinHandleSender<'scope, T> {
    pub fn push(self, result: thread::Result<T>) {
        let data = unsafe { &mut *self.inner.data.get() };
        *data = Some(result);
        self.inner.barrier.notify();
    }
}

pub struct JoinHandleReceiver<'scope, T: 'scope> {
    inner: Arc<JoinHandleInner<'scope, T>>,
    received: bool,
}

impl<'scope, T: 'scope> JoinHandleReceiver<'scope, T> {
    pub fn pop(mut self) -> thread::Result<T> {
        self.inner.barrier.wait().unwrap();
        let data = unsafe { &mut *self.inner.data.get() };
        self.received = true;
        data.take().unwrap()
    }
}

impl<'scope, T: 'scope> Drop for JoinHandleReceiver<'scope, T> {
    fn drop(&mut self) {
        if !self.received {
            self.inner.barrier.wait().unwrap();
            self.received = true;
        }
    }
}

pub fn handle_pair<'scope, T>() -> (JoinHandleSender<'scope, T>, JoinHandleReceiver<'scope, T>)
    where T: 'scope
{
    let inner = Arc::new(JoinHandleInner::new());
    let sender = JoinHandleSender { inner: inner.clone() };
    let receiver = JoinHandleReceiver {
        inner: inner,
        received: false,
    };
    (sender, receiver)
}

#[cfg(test)]
mod test {
    use super::*;

    use scheduler::Scheduler;

    #[test]
    fn test_join_handle_basic() {
        for _ in 0..10 {
            Scheduler::new()
                .run(|| {
                    let (tx, rx) = handle_pair();

                    Scheduler::spawn(move || {
                        tx.push(Ok(1));
                    });

                    let value = rx.pop().unwrap();
                    assert_eq!(value, 1);
                })
                .unwrap();
        }
    }

    #[test]
    fn test_join_handle_basic2() {
        Scheduler::new()
            .run(|| {
                let mut handles = Vec::new();

                for _ in 0..10 {
                    let (tx, rx) = handle_pair();
                    Scheduler::spawn(move || {
                        tx.push(Ok(1));
                    });
                    handles.push(rx);
                }

                for h in handles {
                    assert_eq!(h.pop().unwrap(), 1);
                }
            })
            .unwrap();
    }

    #[test]
    fn test_join_handle_basic3() {
        Scheduler::new()
            .with_workers(4)
            .run(|| {
                let mut handles = Vec::new();

                for _ in 0..10 {
                    let (tx, rx) = handle_pair();
                    Scheduler::spawn(move || {
                        tx.push(Ok(1));
                    });
                    handles.push(rx);
                }

                for h in handles {
                    assert_eq!(h.pop().unwrap(), 1);
                }
            })
            .unwrap();
    }
}
