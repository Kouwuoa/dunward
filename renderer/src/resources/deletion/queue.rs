use super::payload::DeletionPayload;

use std::sync::mpsc::{Receiver, Sender};

pub(crate) struct DeletionQueue {
    sender: Sender<DeletionPayload>,
    receiver: Receiver<DeletionPayload>,
    frame_buckets: Vec<Vec<DeletionPayload>>,
}