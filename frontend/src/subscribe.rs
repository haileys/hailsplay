#![allow(unused)]

use std::{cell::RefCell, rc::Rc};

use yew::Callback;

pub struct Subscriber<T> {
    inner: Rc<RefCell<Inner<T>>>,
}

impl<T: Clone + 'static> Subscriber<T> {
    pub fn broadcast(&self, val: T) {
        let subscribers = {
            let inner = self.inner.borrow_mut();
            inner.subscribers.clone()
        };
        
        for sub in subscribers {
            sub.1.emit(val.clone());
        }
    }

    pub fn subscribe(&self, callback: Callback<T>) -> SubscribeHandle {
        let mut inner = self.inner.borrow_mut();

        // generate next subscriber id
        let id = inner.seq;
        inner.seq += 1;

        inner.subscribers.push((id, callback));

        drop(inner);

        SubscribeHandle {
            inner: self.inner.clone() as Rc<dyn InnerT>,
            id,
        }
    }
}

impl<T> Default for Subscriber<T> {
    fn default() -> Self {
        Subscriber { inner: Default::default() }
    }
}

trait InnerT {
    fn remove_id(&self, id: u64);
}

struct Inner<T> {
    seq: u64,
    subscribers: Vec<(u64, Callback<T>)>,
}

impl<T> Default for Inner<T> {
    fn default() -> Self {
        Inner {
            seq: 0,
            subscribers: Vec::new(),
        }
    }
}

impl<T> InnerT for RefCell<Inner<T>> {
    fn remove_id(&self, id: u64) {
        let mut inner = self.borrow_mut();

        let idx = inner.subscribers.iter()
            .position(|(sub_id, _)| *sub_id == id);

        if let Some(idx) = idx {
            inner.subscribers.swap_remove(idx);
        }
    }
}

pub struct SubscribeHandle {
    inner: Rc<dyn InnerT>,
    id: u64,
}

impl Drop for SubscribeHandle {
    fn drop(&mut self) {
        self.inner.remove_id(self.id);
    }
}
