//! Large-scale pooled GPU memory sub-allocator (Megabuffer).
//!
//! Exposes [`Megabuffer`] and [`AllocatedMegabufferRegion`], providing a general-purpose
//! free-list sub-allocation strategy over large pre-allocated GPU buffers with staging transfers.
//!
//! # Architecture & Purpose
//!
//! In modern GPU-driven rendering, allocating individual `VkBuffer` objects for every mesh,
//! material, or uniform block incurs severe driver overhead, memory fragmentation, and risk
//! of hitting driver allocation limits.
//!
//! A **Megabuffer** solves this by pre-allocating a single giant contiguous buffer up front
//! and sub-allocating smaller regions out of it on demand.
//!
//! # How It Works
//!
//! ### 1. Dual-Buffer Memory Model
//! Each `Megabuffer` owns two underlying buffers of identical capacity:
//! * **Device Buffer** (`AutoPreferDevice`): High-speed, device-local GPU VRAM where draw calls
//!   and shaders read vertex, index, uniform, or storage data.
//! * **Staging Buffer** (`AutoPreferHost`): Host-visible, CPU-writable memory used as a staging
//!   area before transferring data to VRAM.
//!
//! ### 2. Free-List Sub-Allocation Strategy
//! * **Allocation ([`MegabufferExt::allocate_region`])**: Searches the internal free-list for the
//!   first free memory block large enough to fit the requested size (aligned to the Megabuffer's
//!   alignment requirements). It splits the block into an [`AllocatedMegabufferRegion`] and leaves
//!   the remainder in the free list.
//! * **Deallocation ([`MegabufferExt::deallocate_region`])**: When a region is deallocated, it is
//!   returned to the free-list and automatically coalesced (merged) with any immediately adjacent
//!   free regions to prevent memory fragmentation.
//! * **Defragmentation ([`MegabufferExt::defragment`])**: Sorts and merges contiguous free blocks.
//!
//! ### 3. Data Flow & Staging Transfers
//! 1. **Write ([`MegabufferExt::write`])**: CPU writes raw data into the host-visible staging buffer
//!    at the subregion's byte offset.
//! 2. **Upload ([`MegabufferExt::upload`])**: Batches buffer copy commands and executes an immediate
//!    GPU transfer on the transfer queue, copying modified staging regions into the device VRAM buffer.
//!
//! ### 4. RAII Lifetime Management
//! [`AllocatedMegabufferRegion`] implements [`Drop`]. When an allocated region goes out of scope
//! (e.g. when a [`Model`](crate::scene::model::Model) is dropped), it automatically returns its
//! memory range to the parent `Megabuffer`'s free-list.

use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use super::buffer::Buffer;
use crate::commands::transfer::TransferCommandRecorder;

static MEGABUFFER_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Strongly typed Megabuffer identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct MegabufferId(u32);
impl MegabufferId {
    pub fn generate() -> Self {
        Self(MEGABUFFER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

struct PendingMegabufferUpload {
    staging_buffer: Buffer, // Sized only to this specific write payload
    dst_offset: u64,
    size: u64,
}

struct MegabufferState {
    free_regions: Vec<FreeMegabufferRegion>,
    pending_uploads: Vec<PendingMegabufferUpload>,
}

impl MegabufferState {
    /// Initialize a single free region covering the entire capacity
    pub fn new(total_capacity: u64) -> Self {
        Self {
            free_regions: vec![FreeMegabufferRegion {
                offset: 0,
                size: total_capacity,
            }],
            pending_uploads: vec![],
        }
    }

    /// Find a free region that can fit the allocation and splits it into 2 free regions if possible
    /// # Returns
    /// * `Some(FreeMegabufferRegion)` one free region out of the 2 new split free regions that can fit the allocation
    /// * `None` if no free region can fit the allocation
    pub fn carve_free_region(&mut self, alloc_size: u64) -> Option<FreeMegabufferRegion> {
        let (region_index, new_region) = self
            .free_regions
            .iter_mut()
            .enumerate()
            // Find the first free region that can fit the allocation
            .find(|(_, region)| region.size >= alloc_size)
            .map(|(i, region)| {
                // Split the free region into 2 regions:
                // 1. A free region that fits the allocation exactly
                // 2. The remaining free region
                let offset = region.offset;
                region.offset += alloc_size;
                region.size -= alloc_size;
                (
                    // Index of the remaining free region
                    i,
                    // The free region that fits the allocation exactly,
                    // ready to be inserted into the free regions vector
                    FreeMegabufferRegion {
                        offset,
                        size: alloc_size,
                    },
                )
            })?;

        // Insert the new free region into the free regions vector
        if self.free_regions[region_index].size == 0 {
            self.free_regions[region_index] = new_region.clone();
        } else {
            self.free_regions.insert(region_index, new_region.clone());
        }

        Some(new_region)
    }

    pub fn reclaim_region(&mut self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        let mut left_index = None; // Some if there is a free region to the left of the deallocated region
        let mut right_index = None; // Some if there is a free region to the right of the deallocated region

        for (i, free_region) in self.free_regions.iter().enumerate() {
            if free_region.offset + free_region.size == region.offset {
                left_index = Some(i);
            } else if region.offset + region.size == free_region.offset {
                right_index = Some(i);
            }
        }

        match (left_index, right_index) {
            // Case A: Merges with both left and right adjacent free regions
            (Some(left), Some(right)) => {
                self.free_regions[left].size += region.size + self.free_regions[right].size;
                self.free_regions.remove(right);
            }
            // Case B: Merges with only the left adjacent free region
            (Some(left), None) => {
                self.free_regions[left].size += region.size;
            }
            // Case C: Merges with only the right adjacent free region
            (None, Some(right)) => {
                self.free_regions[right].offset = region.offset;
                self.free_regions[right].size += region.size;
            }
            // Case D: No adjacent free regions, so insert and keep sorted by offset
            (None, None) => {
                let region = FreeMegabufferRegion {
                    offset: region.offset,
                    size: region.size,
                };
                self.free_regions.push(region);
                self.free_regions.sort_by_key(|r| r.offset);
            }
        }

        region.size = 0; // Mark the region as invalid by setting size to 0

        Ok(())
    }

    fn defragment_free_regions(&mut self) -> Result<()> {
        self.free_regions.sort_by_key(|r| r.offset);

        // Merge adjacent free regions
        let mut i = 0;
        while i < self.free_regions.len() - 1 {
            if self.free_regions[i].offset + self.free_regions[i].size
                == self.free_regions[i + 1].offset
            {
                self.free_regions[i].size += self.free_regions[i + 1].size;
                self.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }

        Ok(())
    }
}

pub(crate) struct Megabuffer {
    id: MegabufferId,

    /// Immutable GPU buffer handle with no locks needed to read/bind
    buffer: Arc<Buffer>,
    alignment: u64,
    state: Arc<Mutex<MegabufferState>>,

    device: Arc<ash::Device>,
    mem_allocator: Arc<Mutex<vk_mem::Allocator>>,
    upload_recorder: Arc<TransferCommandRecorder>,
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
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer: Arc<TransferCommandRecorder>,
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
            memory_allocator.clone(),
            device.clone(),
        )?);

        Ok(Megabuffer {
            id: MegabufferId::generate(),

            buffer,
            alignment,
            state: Arc::new(Mutex::new(MegabufferState {
                free_regions: vec![FreeMegabufferRegion { offset: 0, size }],
                pending_uploads: vec![],
            })),

            device,
            mem_allocator: memory_allocator,
            upload_recorder: transfer,
        })
    }

    /// Find a fitting free region, split it, and return the allocated region (locks the free-list)
    pub fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let aligned_size = self.aligned_size(size);
        let mut state = self.state.lock().map_err(|e| eyre!(e.to_string()))?;
        // Find fitting free region
        let free_region = state
            .carve_free_region(aligned_size)
            .ok_or_eyre("Megabuffer out of memory")?;
        // Convert free region to allocated region
        Ok(AllocatedMegabufferRegion {
            offset: free_region.offset,
            size: free_region.size,
            parent_megabuffer_state: self.state.clone(),
        })
    }

    /// Deallocate an allocated region and merge it with adjacent free regions if possible.
    pub fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if region.size == 0 {
            return Err(eyre!(
                "Cannot deallocate region with size 0. This region was likely already deallocated."
            ));
        }

        let mut state = self.state.lock().map_err(|e| eyre!(e.to_string()))?;
        state.reclaim_region(region)
    }

    pub fn defragment(&self) -> Result<()> {
        let mut state = self.state.lock().map_err(|e| eyre!(e.to_string()))?;
        state.defragment_free_regions()
    }

    fn upload(&self) -> Result<()> {
        let mut guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        // Don't upload if there are no pending uploads
        if guard.pending_uploads.is_empty() {
            return Ok(());
        }

        // Drain all queued pending uploads
        let uploads = std::mem::take(&mut guard.pending_uploads);

        // Record all copies into ONE single transfer command buffer
        guard.upload_recorder.immediate_submit(
            |cmd: vk::CommandBuffer, device: &ash::Device| {
                let dst_buffer = guard.buffer.lock().map_err(|e| eyre!(e.to_string()))?;
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
                            dst_buffer.buffer,
                            &[copy_region],
                        );
                    }
                }

                Ok(())
            },
        )?;

        Ok(())
    }

    fn write<T>(&self, data: &[T], region: &AllocatedMegabufferRegion) -> Result<()>
    where
        T: Copy,
    {
        let data_size = size_of_val(data) as u64;
        if data_size > region.size {
            return Err(eyre!("Data too large for region"));
        }

        let mut inner_guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        // Allocate a temporary staging buffer ONLY for this data
        let mut staging_buffer = Buffer::new(
            data_size,
            inner_guard.alignment,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            inner_guard.mem_allocator.clone(),
            inner_guard.device.clone(),
        )?;

        let _ = staging_buffer.write(data, 0)?;
        inner_guard.pending_uploads.push(PendingMegabufferUpload {
            staging_buffer,
            dst_offset: region.offset,
            size: data_size,
        });

        Ok(())
    }

    fn aligned_size(&self, size: u64) -> u64 {
        (size + self.alignment - 1) & !(self.alignment - 1)
    }
}

#[derive(Debug, Clone)]
struct FreeMegabufferRegion {
    offset: u64,
    size: u64,
}

pub(crate) struct AllocatedMegabufferRegion {
    offset: u64,
    /// Size of the allocated region. This is 0 when the region is deallocated.
    size: u64,
    parent_megabuffer_state: Arc<Mutex<MegabufferState>>,
}

impl AllocatedMegabufferRegion {
    pub fn write<T>(&mut self, data: &[T]) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        self.parent_megabuffer.as_ref().unwrap().write(data, self)
    }

    pub fn suballocate_region(&mut self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let size = self
            .parent_megabuffer
            .as_ref()
            .unwrap()
            .aligned_size(size)?;

        if size > self.size {
            return Err(eyre!("Subregion size too large"));
        }
        if size == 0 {
            return Err(eyre!("Subregion size cannot be zero"));
        }
        if size == self.size {
            return Err(eyre!("Subregion size cannot be the parent region"));
        }

        let subregion = AllocatedMegabufferRegion {
            offset: self.offset + (self.size - size),
            size,
            parent_megabuffer: self.parent_megabuffer.clone(),
        };
        self.size -= size;

        Ok(subregion)
    }

    pub fn belongs_to_same_megabuffer(&self, other: &Self) -> bool {
        self.parent_megabuffer == other.parent_megabuffer
    }

    pub fn belongs_to_megabuffer(&self, megabuffer: &Megabuffer) -> bool {
        self.parent_megabuffer.as_ref().unwrap() == megabuffer
    }

    pub fn is_adjacent_to(&self, other: &Self) -> bool {
        if !self.belongs_to_same_megabuffer(other) {
            return false;
        }

        let (left_offset, left_size, right_offset) = if self.offset < other.offset {
            (self.offset, self.size, other.offset)
        } else {
            (other.offset, other.size, self.offset)
        };

        left_offset + left_size == right_offset
    }

    pub fn merge_adjacent_region(&mut self, other: Self) -> Result<()> {
        if !self.belongs_to_same_megabuffer(&other) {
            return Err(eyre!(
                "Cannot combine regions belonging to different megabuffers"
            ));
        }
        if !self.is_adjacent_to(&other) {
            return Err(eyre!("Cannot combine regions that are not adjacent"));
        }

        let (new_offset, new_size) = {
            let (left_offset, left_size, right_size) = if self.offset < other.offset {
                (self.offset, self.size, other.size)
            } else {
                (other.offset, other.size, self.size)
            };

            let new_offset = left_offset;
            let new_size = left_size + right_size;

            (new_offset, new_size)
        };

        self.offset = new_offset;
        self.size = new_size;

        Ok(())
    }
}

impl Drop for AllocatedMegabufferRegion {
    fn drop(&mut self) {
        if self.size > 0 {
            if let Ok(mut state) = self.parent_megabuffer_state.lock() {
                state.deallocate_region(self)
            }
        }
    }
}
