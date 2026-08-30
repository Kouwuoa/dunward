use super::payload::DeletionPayload;

use std::ops::{Deref, DerefMut};
use std::sync::mpsc;

#[repr(transparent)]
pub(crate) struct DeletionReceiver(pub mpsc::Receiver<DeletionPayload>);

impl Deref for DeletionReceiver {
    type Target = mpsc::Receiver<DeletionPayload>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DeletionReceiver {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}