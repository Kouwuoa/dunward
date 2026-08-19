use super::region::AllocatedMegabufferRegion;
use crate::resources::buffer::Buffer;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub(crate) struct MegabufferWriteRecord {
    staging_buffer: Buffer,
    dst_offset: u64,
    size: u64,
}

pub(crate) struct MegabufferWriter {
    alignment: u64,
    /// Lock-free sending endpoint (MPSC) for queued staging payloads awaiting GPU upload.
    ///
    /// Cloned and shared across all `AllocatedMegabufferRegion`s.
    /// When `AllocatedMegabufferRegion::write` is called from any worker thread, it sends a new `MegabufferWriteRecord` into the channel.
    /// The records accumulate in host-visible memory until drained by the `Megabuffer`'s receiver during `Megabuffer::upload`.
    write_sender: Sender<MegabufferWriteRecord>,
    device: Arc<ash::Device>,
    mem_allocator: Arc<Mutex<vk_mem::Allocator>>,
}

impl MegabufferWriter {
    pub fn queue_write<T>(&self, data: &[T], region: &AllocatedMegabufferRegion) -> Result<()>
    where
        T: Copy,
    {
        let data_size = size_of_val(data) as u64;
        if data_size > region.size {
            return Err(eyre!("Data too large for region"));
        }

        // Allocate a temporary host-visible staging buffer ONLY for this data
        let mut staging_buffer = Buffer::new(
            data_size,
            self.alignment,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            self.mem_allocator.clone(),
            self.device.clone(),
        )?;

        // Write CPU bytes into the staging buffer
        let _ = staging_buffer.write(data, 0)?;

        // Send write record down the lock-free MPSC channel
        self.write_sender.send(
            MegabufferWriteRecord {
                staging_buffer,
                dst_offset: region.offset,
                size: data_size,
            }
        )?;

        Ok(())
    }
}
