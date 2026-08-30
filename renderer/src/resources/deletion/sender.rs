use super::payload::DeletionPayload;
use std::sync::mpsc::Sender;

pub(crate) struct DeletionSender<T = DeletionPayload> {
    inner: Sender<DeletionPayload>,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Into<DeletionPayload>> DeletionSender<T> {
    pub fn new(inner: Sender<DeletionPayload>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn send(&self, payload: T) {
        self.inner.send(payload.into()).unwrap();
    }

    pub fn clone<U: Into<DeletionPayload>>(&self) -> DeletionSender<U> {
        DeletionSender {
            inner: self.inner.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}
