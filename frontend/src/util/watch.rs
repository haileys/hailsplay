use std::{rc::Rc, cell::RefCell, task::{Waker, Context, Poll}, pin::Pin};

use futures::Stream;

pub fn channel<T>(init: T) -> (Sender<T>, Receiver<T>) {
    todo!();
}

struct Shared<T> {
    value: Rc<T>,
    wakers: Vec<Waker>,
}

type SharedRef<T> = Rc<RefCell<Shared<T>>>;

pub struct Sender<T> {
    shared: SharedRef<T>,
}

pub struct Receiver<T> {
    value: Option<Rc<T>>,
    shared: SharedRef<T>,
}

impl<T> Sender<T> {
    pub fn send(&self, value: T) {
        let mut shared = self.shared.borrow_mut();
        shared.value = Rc::new(value);
        for waker in shared.wakers.drain(..) {
            waker.wake();
        }
    }
}

impl<T> Receiver<T> {
}

impl<T> Stream for Receiver<T> {
    type Item = Rc<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Rc<T>>> {
        let mut shared = self.shared.borrow_mut();

        let shared_ptr = Some(Rc::as_ptr(&shared.value));
        let our_ptr = self.value.as_ref().map(Rc::as_ptr);

        if shared_ptr == our_ptr {
            shared.wakers.push(cx.waker().clone());
            return Poll::Pending;
        }

        self.value = Some(shared.value.clone());
        Poll::Ready(shared.value.clone())
    }
}
