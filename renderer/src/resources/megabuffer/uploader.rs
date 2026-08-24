use super::{Megabuffer, MegabufferWriteRecord};
use crate::commands::TransferCommandRecorder;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use std::sync::{Arc, Mutex, mpsc};

pub(crate) struct MegabufferUploader {
    write_receiver: mpsc::Receiver<MegabufferWriteRecord>,
    upload_recorder: Arc<Mutex<TransferCommandRecorder>>,
}

impl MegabufferUploader {
    pub fn new(
        write_receiver: mpsc::Receiver<MegabufferWriteRecord>,
        upload_recorder: Arc<Mutex<TransferCommandRecorder>>,
    ) -> Self {
        Self {
            write_receiver,
            upload_recorder,
        }
    }

    /// Batches and transfers all queued pending uploads to the GPU
    pub fn upload(&mut self, megabuffer: &Megabuffer) -> Result<()> {
        let upload_records: Vec<MegabufferWriteRecord> = self.write_receiver.try_iter().collect();

        // Lock-free GPU transfer submission that
        // records all copy operations into ONE single transfer command buffer
        self.upload_recorder
            .lock()
            .map_err(|_| eyre!("Failed to lock upload recorder"))?
            .immediate_submit(|cmd: vk::CommandBuffer, device: &ash::Device| {
                for upload in upload_records {
                    let copy_region = vk::BufferCopy {
                        src_offset: 0,
                        dst_offset: upload.dst_offset,
                        size: upload.size,
                    };
                    unsafe {
                        device.cmd_copy_buffer(
                            cmd,
                            upload.staging_buffer.raw(),
                            megabuffer.buffer.raw(),
                            &[copy_region],
                        );
                    }
                }

                Ok(())
            })?;

        Ok(())
    }
}
