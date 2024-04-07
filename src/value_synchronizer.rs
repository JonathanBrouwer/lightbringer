use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};
use embassy_sync::blocking_mutex::Mutex;

use embassy_sync::blocking_mutex::raw::RawMutex;
use embassy_sync::waitqueue::MultiWakerRegistration;

pub struct ValueSynchronizer<const WATCHER_COUNT: usize, M: RawMutex, T>(
    Mutex<M, RefCell<ValueSynchronizerData<WATCHER_COUNT, T>>>,
);

struct ValueSynchronizerData<const WATCHER_COUNT: usize, T> {
    value: T,
    wakers: MultiWakerRegistration<WATCHER_COUNT>,
    counter: usize,
}

impl<const WATCHER_COUNT: usize, M: RawMutex, T> ValueSynchronizer<WATCHER_COUNT, M, T> {
    pub fn new(value: T) -> Self {
        Self(Mutex::new(RefCell::new(ValueSynchronizerData {
            value,
            wakers: MultiWakerRegistration::new(),
            counter: 0,
        })))
    }

    /// f should not block
    pub fn read<U>(&self, f: impl FnOnce(&T) -> U) -> U {
        self.0.lock(|v| f(&v.borrow().value))
    }

    pub fn read_clone(&self) -> T
    where
        T: Clone,
    {
        self.read(|v| v.clone())
    }

    pub fn update(&self, f: impl FnOnce(&mut T)) {
        self.0.lock(|inner| {
            let mut s = inner.borrow_mut();
            f(&mut s.value);
            s.counter += 1;
            s.wakers.wake()
        });
    }

    pub async fn write(&self, v_new: T) {
        self.update(|v| *v = v_new)
    }

    pub fn watch(&self) -> Watcher<WATCHER_COUNT, M, T> {
        Watcher {
            last_counter: self.0.lock(|v| v.borrow().counter),
            synchronizer: self,
        }
    }
}

pub struct Watcher<'a, const WATCHER_COUNT: usize, M: RawMutex, T> {
    synchronizer: &'a ValueSynchronizer<WATCHER_COUNT, M, T>,
    last_counter: usize, //TODO is this necessary
}

impl<'a, const WATCHER_COUNT: usize, M: RawMutex, T> Watcher<'a, WATCHER_COUNT, M, T> {
    // pub fn watch<'s>(&'s mut self) -> WatcherFuture<'s, 'a, WATCHER_COUNT, M, T> {
    //     WatcherFuture(self)
    // }

    pub fn read<'s>(&'s mut self) -> ReaderFuture<'s, 'a, WATCHER_COUNT, M, T>
    where
        T: Clone,
    {
        ReaderFuture(self)
    }

    pub async fn skip(&mut self) {
        self.last_counter = self.synchronizer.0.lock(|v| v.borrow().counter);
    }
}

// TODO fix
// pub struct WatcherFuture<'s, 'a, const WATCHER_COUNT: usize, M: RawMutex, T>(&'s mut Watcher<'a, WATCHER_COUNT, M, T>);
//
// impl<'s, 'a, const WATCHER_COUNT: usize, M: RawMutex, T> Future for WatcherFuture<'s, 'a, WATCHER_COUNT, M, T> {
//     type Output = ();
//
//     fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
//         match pin!(self.0.synchronizer.0.lock()).poll(cx) {
//             Poll::Pending => {
//                 panic!("Watcher should be able to lock ")
//             },
//             Poll::Ready(guard) => if guard.counter != self.0.last_counter {
//                 self.0.last_counter = guard.counter;
//                 Poll::Ready(())
//             } else {
//                 self.0.synchronizer.0.lock().poll
//                 Poll::Pending
//             }
//         }
//     }
// }

pub struct ReaderFuture<'s, 'a, const WATCHER_COUNT: usize, M: RawMutex, T: Clone>(
    &'s mut Watcher<'a, WATCHER_COUNT, M, T>,
);

impl<'s, 'a, const WATCHER_COUNT: usize, M: RawMutex, T: Clone> Future
    for ReaderFuture<'s, 'a, WATCHER_COUNT, M, T>
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.synchronizer.0.lock(|inner| {
            let mut s = inner.borrow_mut();
            if s.counter != self.0.last_counter {
                self.0.last_counter = s.counter;
                Poll::Ready(s.value.clone())
            } else {
                s.wakers.register(cx.waker());
                Poll::Pending
            }
        })
    }
}
