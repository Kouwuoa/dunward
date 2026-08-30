use super::payload::DeletionPayload;
use super::receiver::DeletionReceiver;
use super::sender::DeletionSender;

use crate::gpu::Gpu;

pub(crate) struct DeletionQueue {
    sender: DeletionSender,
    receiver: DeletionReceiver,
    frame_buckets: Vec<Vec<DeletionPayload>>,
}

impl DeletionQueue {
    pub fn new(frames_in_flight: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender: DeletionSender::new(sender),
            receiver: DeletionReceiver(receiver),
            frame_buckets: std::iter::repeat_with(Vec::new).take(frames_in_flight).collect(),
        }
    }

    pub fn get_sender(&self) -> DeletionSender {
        self.sender.clone()
    }

    pub fn flush_all(&mut self, gpu: &Gpu) {
        while let Ok(payload) = self.receiver.try_recv() {
            payload.destroy(gpu)
        }
        for bucket in &mut self.frame_buckets {
            for payload in bucket.drain(..) {
                payload.destroy(gpu);
            }
        }
    }

    /// Gather all resources scheduled for deletion into the frame bucket for the given frame slot.
    pub fn collect_pending(&mut self, frame_slot: usize) {
        while let Ok(payload) = self.receiver.try_recv() {
            self.frame_buckets[frame_slot].push(payload);
        }
    }

    /// Destroy all resources scheduled for deletion currently in the frame bucket for the given frame slot.
    /// NOTE: This function does nothing if the frame bucket is empty. Call [`DeletionQueue::collect_pending`] first to fill the frame bucket.
    pub fn destroy_pending(&mut self, frame_slot: usize, gpu: &Gpu) {
        while let Some(payload) = self.frame_buckets[frame_slot].pop() {
            payload.destroy(gpu);
        }
    }
}