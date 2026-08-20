mod freelist;
mod region;
mod writer;
mod uploader;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::atomic::AtomicU32;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, MutexGuard, mpsc};

use super::buffer::Buffer;
use freelist::MegabufferFreeList;
use region::AllocatedMegabufferRegion;
use writer::{MegabufferWriteRecord, MegabufferWriter};

static MEGABUFFER_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Strongly typed Megabuffer identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MegabufferId(u32);
impl MegabufferId {
    pub fn generate() -> Self {
        Self(MEGABUFFER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

pub(crate) struct Megabuffer {
    id: MegabufferId,
    buffer: Arc<Buffer>,
    free_list: Arc<Mutex<MegabufferFreeList>>,
    writer: Arc<MegabufferWriter>,
    write_receiver: Receiver<MegabufferWriteRecord>,

    device: Arc<ash::Device>,
}

impl PartialEq for Megabuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Megabuffer {
    fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
        mem_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Megabuffer> {
        log::info!(
            "Creating Megabuffer with size: {}, alignment: {}, usage: {:?}",
            size,
            alignment,
            buf_usage
        );

        let mem_usage = vk_mem::MemoryUsage::AutoPreferDevice;
        let buffer = Arc::new(Buffer::new(
            size,
            alignment,
            buf_usage,
            mem_usage,
            false,
            mem_allocator.clone(),
            device.clone(),
        )?);

        let id = MegabufferId::generate();
        let free_list = Arc::new(Mutex::new(MegabufferFreeList::new(id, size)));
        let (write_sender, write_receiver) = mpsc::channel();
        let writer = Arc::new(MegabufferWriter::new(
            id,
            alignment,
            write_sender,
            device.clone(),
            mem_allocator.clone(),
        ));

        Ok(Megabuffer {
            id,
            buffer,
            free_list,
            writer,
            write_receiver,
            device,
        })
    }

    /// Find a fitting free region, split it, and return the allocated region (locks the free-list)
    pub fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let aligned_size = self.aligned_size(size);
        let mut state = self.lock()?;
        // Find fitting free region
        let free_region = state
            .carve_free_region(aligned_size)
            .ok_or_eyre("Megabuffer out of memory: no suitable free region found")?;
        // Convert free region to allocated region
        Ok(AllocatedMegabufferRegion {
            offset: free_region.offset,
            size: free_region.size,
            parent_megabuffer: *self.clone(),
        })
    }

    /// Deallocate an allocated region and merge it with adjacent free regions if possible.
    pub fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if region.size == 0 {
            return Err(eyre!(
                "Cannot deallocate region with size 0. This region was likely already deallocated."
            ));
        }

        let mut state = self.lock()?;
        state.reclaim_region(region)
    }

    pub fn defragment(&self) -> Result<()> {
        let mut state = self.lock()?;
        state.defragment_free_regions()
    }

    /// Batches and transfers all queued pending uploads to the GPU
    pub fn upload(&self) -> Result<()> {
        // Lock briefly to drain the pending uploads
        let uploads = {
            let mut state = self.lock()?;

            // Don't upload if there are no pending uploads
            if !state.has_pending_uploads() {
                return Err(eyre!("No pending uploads to upload"));
            }

            // Drain all queued pending uploads
            state.drain_pending_uploads()
        }; // Lock released here!

        // Lock-free GPU transfer submission that
        // records all copy operations into ONE single transfer command buffer
        self.upload_recorder
            .immediate_submit(|cmd: vk::CommandBuffer, device: &ash::Device| {
                for upload in uploads {
                    let copy_region = vk::BufferCopy {
                        src_offset: 0,
                        dst_offset: upload.dst_offset,
                        size: upload.size,
                    };
                    unsafe {
                        device.cmd_copy_buffer(
                            cmd,
                            upload.staging_buffer.buffer,
                            self.buffer.buffer,
                            &[copy_region],
                        );
                    }
                }

                Ok(())
            })?;

        Ok(())
    }

    /// Writes data into a temporary staging buffer and queues it for GPU upload
    pub fn write<T>(&self, data: &[T], region: &AllocatedMegabufferRegion) -> Result<()>
    where
        T: Copy,
    {}

    fn aligned_size(&self, size: u64) -> u64 {
        (size + self.alignment - 1) & !(self.alignment - 1)
    }

    fn lock(&self) -> Result<MutexGuard<MegabufferState>> {
        self.state.lock().map_err(|e| eyre!(e.to_string()))
    }
}
