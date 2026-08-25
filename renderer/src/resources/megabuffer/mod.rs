pub(crate) mod region;
mod freelist;
mod uploader;
mod writer;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex, mpsc};

use super::buffer::Buffer;
use crate::commands::TransferCommandRecorder;
use crate::gpu::Gpu;
use crate::resources::megabuffer::uploader::MegabufferUploader;
use freelist::MegabufferFreeList;
use region::AllocatedMegabufferRegion;
use writer::{MegabufferWriteRecord, MegabufferWriter};

pub(crate) fn aligned_size(size: u64, alignment: u64) -> u64 {
    (size + alignment - 1) & !(alignment - 1)
}

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
    buffer: Buffer,
    free_list: MegabufferFreeList,
    writer: Arc<MegabufferWriter>,
    uploader: MegabufferUploader,
    gpu: Arc<Gpu>,
}

impl PartialEq for Megabuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Megabuffer {
    pub fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
        gpu: Arc<Gpu>,
        upload_recorder: Arc<Mutex<TransferCommandRecorder>>,
    ) -> Result<Megabuffer> {
        log::info!(
            "Creating Megabuffer with size: {}, alignment: {}, usage: {:?}",
            size,
            alignment,
            buf_usage
        );

        let mem_usage = vk_mem::MemoryUsage::AutoPreferDevice;
        let buffer = Buffer::new(size, alignment, buf_usage, mem_usage, false, gpu.clone())?;

        let id = MegabufferId::generate();
        let free_list = MegabufferFreeList::new(id, size);
        let (write_sender, write_receiver) = mpsc::channel();
        let writer = Arc::new(MegabufferWriter::new(
            id,
            alignment,
            write_sender,
            gpu.clone(),
        ));
        let uploader = MegabufferUploader::new(write_receiver, upload_recorder);

        Ok(Self {
            id,
            buffer,
            free_list,
            writer,
            uploader,
            gpu,
        })
    }

    /// Find a fitting free region, split it, and return the allocated region (locks the free-list)
    pub fn allocate_region(&mut self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let aligned_size = self.aligned_size(size);
        // Find fitting free region
        let free_region = self
            .free_list
            .carve_free_region(aligned_size)
            .ok_or_eyre("Megabuffer out of memory: no suitable free region found")?;
        // Convert free region to allocated region
        Ok(AllocatedMegabufferRegion::new(
            free_region.offset,
            free_region.size,
            self.buffer.alignment(),
            self.id,
            self.writer.clone(),
        ))
    }

    /// Deallocate an allocated region and merge it with adjacent free regions if possible.
    pub fn deallocate_region(&mut self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if region.size == 0 {
            return Err(eyre!(
                "Cannot deallocate region with size 0. This region was likely already deallocated."
            ));
        }

        self.free_list.reclaim_region(region)
    }

    pub fn defragment(&mut self) {
        self.free_list.defragment_free_regions()
    }

    fn aligned_size(&self, size: u64) -> u64 {
        aligned_size(size, self.buffer.alignment())
    }
}
