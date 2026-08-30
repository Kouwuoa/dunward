use super::AllocatedMegabufferRegion;
use super::Buffer;
use super::MegabufferId;
use crate::gpu::Gpu;

use crate::resources::deletion::payload::BufferDeletionPayload;
use crate::resources::deletion::sender::DeletionSender;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use std::sync::{Arc, mpsc};

pub(crate) struct MegabufferWriteRecord {
    pub staging_buffer: Buffer,
    pub dst_offset: u64,
    pub size: u64,
}

pub(crate) struct MegabufferWriter {
    megabuffer_id: MegabufferId,
    alignment: u64,
    /// Lock-free sending endpoint (MPSC) for queued staging payloads awaiting GPU upload.
    ///
    /// Cloned and shared across all `AllocatedMegabufferRegion`s.
    /// When `AllocatedMegabufferRegion::write` is called from any worker thread, it sends a new `MegabufferWriteRecord` into the channel.
    /// The records accumulate in host-visible memory until drained by the `Megabuffer`'s receiver during `Megabuffer::upload`.
    write_sender: mpsc::Sender<MegabufferWriteRecord>,
    gpu: Arc<Gpu>,
    staging_buffer_deletion_sender: DeletionSender<BufferDeletionPayload>,
}

impl MegabufferWriter {
    pub fn new(
        megabuffer_id: MegabufferId,
        alignment: u64,
        write_sender: mpsc::Sender<MegabufferWriteRecord>,
        gpu: Arc<Gpu>,
        staging_buffer_deletion_sender: DeletionSender<BufferDeletionPayload>,
    ) -> Self {
        Self {
            megabuffer_id,
            alignment,
            write_sender,
            gpu,
            staging_buffer_deletion_sender,
        }
    }

    /// Writes a slice of data to a specified region within a megabuffer.
    ///
    /// This function handles copying the provided data to a temporary host-visible staging
    /// buffer and then enqueues a write operation to the megabuffer using an internal MPSC channel.
    pub fn write<T>(&self, data: &[T], region: &AllocatedMegabufferRegion) -> Result<()>
    where
        T: Copy,
    {
        // Do not allow regions that do not belong to this megabuffer
        if !region.belongs_to_megabuffer_id(self.megabuffer_id) {
            return Err(eyre!(
                "Region does not belong to the megabuffer associated with this MegabufferWriter"
            ));
        }

        let data_size = size_of_val(data) as u64;
        if data_size > region.size() {
            return Err(eyre!("Data too large for region"));
        }

        // Allocate a temporary host-visible staging buffer ONLY for this data
        let mut staging_buffer = Buffer::new(
            data_size,
            self.alignment,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            self.gpu.clone(),
            self.staging_buffer_deletion_sender.clone(),
        )?;

        // Write CPU bytes into the staging buffer
        let _ = staging_buffer.write(data, 0)?;

        // Send write record down the lock-free MPSC channel
        self.write_sender.send(MegabufferWriteRecord {
            staging_buffer,
            dst_offset: region.offset(),
            size: data_size,
        })?;

        Ok(())
    }
}
