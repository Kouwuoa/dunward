use super::MegabufferWriteRecord;
use crate::commands::TransferCommandRecorder;
use std::sync::{Arc, mpsc};

pub(crate) struct MegabufferUploader {
    write_receiver: mpsc::Receiver<MegabufferWriteRecord>,
    upload_recorder: Arc<TransferCommandRecorder>,
}
