use super::buffer::Buffer;
use crate::context::commands::TransferCommandEncoder;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

static MEGABUFFER_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(crate) struct Megabuffer {
    inner: Arc<Mutex<MegabufferInner>>,
    id: usize,
}

impl Clone for Megabuffer {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            id: self.id,
        }
    }
}

impl PartialEq for Megabuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub(crate) trait MegabufferExt {
    fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer: Arc<TransferCommandEncoder>,
    ) -> Result<Megabuffer>;
    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion>;
    fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()>;
    fn defragment(&self) -> Result<()>;
    fn upload(&self) -> Result<()>;
    fn write<T>(
        &self,
        data: &[T],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy;
    fn aligned_size(&self, size: u64) -> Result<u64>;
}

impl MegabufferExt for Megabuffer {
    fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,

        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer: Arc<TransferCommandEncoder>,
    ) -> Result<Megabuffer> {
        log::info!(
            "Creating Megabuffer with size: {}, alignment: {}, usage: {:?}",
            size,
            alignment,
            buf_usage
        );

        let mem_usage = vk_mem::MemoryUsage::AutoPreferDevice;
        let buffer = Arc::new(Mutex::new(Buffer::new(
            size,
            alignment,
            buf_usage,
            mem_usage,
            false,
            memory_allocator.clone(),
            device.clone(),
        )?));

        let staging_buffer = Arc::new(Mutex::new(Buffer::new(
            size,
            alignment,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            memory_allocator.clone(),
            device.clone(),
        )?));

        let id = MEGABUFFER_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(Megabuffer {
            inner: Arc::new(Mutex::new(MegabufferInner {
                buffer,
                staging_buffer,
                free_regions: vec![FreeMegabufferRegion { offset: 0, size }],
                alignment,
                transfer,
                id,
                mem_allocator: memory_allocator,
                device,
            })),
            id,
        })
    }

    fn allocate_region(&self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let mut guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        let aligned_size = guard.aligned_size(size);
        let free_region_index = guard
            .find_free_region_for_allocation(aligned_size)
            .ok_or_eyre("Failed to find free region for allocation")?;

        // Remove the free region from the free regions vector
        let free_region = guard.free_regions.remove(free_region_index);
        let allocated_region = AllocatedMegabufferRegion {
            offset: free_region.offset,
            size: free_region.size,
            parent_megabuffer: Some(self.clone()),
        };

        Ok(allocated_region)
    }

    /// Deallocate a region and merge it with adjacent free regions if possible.
    fn deallocate_region(&self, region: &mut AllocatedMegabufferRegion) -> Result<()> {
        if region.size == 0 {
            return Err(eyre!(
                "Cannot deallocate region with size 0. This region was likely already deallocated."
            ));
        }

        let mut guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        let mut left_index = None; // Some if there is a free region to the left of the deallocated region
        let mut right_index = None; // Some if there is a free region to the right of the deallocated region

        for (i, free_region) in guard.free_regions.iter().enumerate() {
            if free_region.offset + free_region.size == region.offset {
                left_index = Some(i);
            } else if region.offset + region.size == free_region.offset {
                right_index = Some(i);
            }
        }

        match (left_index, right_index) {
            (Some(left), Some(right)) => {
                guard.free_regions[left].size += region.size + guard.free_regions[right].size;
                guard.free_regions.remove(right);
            }
            (Some(left), None) => {
                guard.free_regions[left].size += region.size;
            }
            (None, Some(right)) => {
                guard.free_regions[right].offset = region.offset;
                guard.free_regions[right].size += region.size;
            }
            (None, None) => {
                let region = FreeMegabufferRegion {
                    offset: region.offset,
                    size: region.size,
                };
                guard.free_regions.push(region);
                guard.free_regions.sort_by_key(|r| r.offset);
            }
        }

        region.size = 0; // Mark the region as invalid by setting size to 0

        Ok(())
    }

    fn defragment(&self) -> Result<()> {
        let mut guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        guard.free_regions.sort_by_key(|r| r.offset);

        // Merge adjacent free regions
        let mut i = 0;
        while i < guard.free_regions.len() - 1 {
            if guard.free_regions[i].offset + guard.free_regions[i].size
                == guard.free_regions[i + 1].offset
            {
                guard.free_regions[i].size += guard.free_regions[i + 1].size;
                guard.free_regions.remove(i + 1);
            } else {
                i += 1;
            }
        }

        Ok(())
    }

    fn upload(&self) -> Result<()> {
        let guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        guard
            .transfer
            .immediate_submit(|cmd: vk::CommandBuffer, device: &ash::Device| {
                let copy_regions = guard
                    .free_regions
                    .iter()
                    .map(|region| vk::BufferCopy {
                        src_offset: region.offset,
                        dst_offset: region.offset,
                        size: region.size,
                    })
                    .collect::<Vec<vk::BufferCopy>>();

                let src_guard = guard
                    .staging_buffer
                    .lock()
                    .map_err(|e| eyre!(e.to_string()))?;
                let dst_guard = guard.buffer.lock().map_err(|e| eyre!(e.to_string()))?;

                unsafe {
                    device.cmd_copy_buffer(cmd, src_guard.buffer, dst_guard.buffer, &copy_regions);
                }

                Ok(())
            })?;

        Ok(())
    }

    fn write<T>(
        &self,
        data: &[T],
        region: &AllocatedMegabufferRegion,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if size_of_val(data) as u64 > region.size {
            return Err(eyre!("Data too large for region"));
        }

        let inner_guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        let mut staging_guard = inner_guard
            .staging_buffer
            .lock()
            .map_err(|e| eyre!(e.to_string()))?;

        staging_guard.write(data, region.offset as usize)
    }

    fn aligned_size(&self, size: u64) -> Result<u64> {
        let guard = self.inner.lock().map_err(|e| eyre!(e.to_string()))?;

        Ok(guard.aligned_size(size))
    }
}

struct MegabufferInner {
    id: usize,

    buffer: Arc<Mutex<Buffer>>,
    staging_buffer: Arc<Mutex<Buffer>>,
    free_regions: Vec<FreeMegabufferRegion>,
    alignment: u64,

    mem_allocator: Arc<Mutex<vk_mem::Allocator>>,
    device: Arc<ash::Device>,
    transfer: Arc<TransferCommandEncoder>,
}

impl MegabufferInner {
    fn aligned_size(&self, size: u64) -> u64 {
        (size + self.alignment - 1) & !(self.alignment - 1)
    }

    /// Find a free region that can fit the allocation and splits it into 2 free regions if possible
    /// Returns the index of the free region that fits the allocation
    fn find_free_region_for_allocation(&mut self, alloc_size: u64) -> Option<usize> {
        let (region_index, new_region) = self
            .free_regions
            .iter_mut()
            .enumerate()
            // Find the first free region that can fit the allocation
            .find(|(_, region)| {
                    log::error!(
                        "Free region: {} < {}",
                        region.size,
                        alloc_size
                    );
                region.size >= alloc_size
            })
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
            self.free_regions[region_index] = new_region;
        } else {
            self.free_regions.insert(region_index, new_region);
        }

        Some(region_index)
    }
}

impl PartialEq for MegabufferInner {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug)]
pub(crate) struct FreeMegabufferRegion {
    offset: u64,
    size: u64,
}

pub(crate) struct AllocatedMegabufferRegion {
    offset: u64,
    /// Size of the allocated region. This is 0 when the region is deallocated.
    size: u64,
    /// Note that this is only an `Option` to allow for the region to be dropped
    /// without needing to keep a reference to the Megabuffer.
    /// This means that the only time this field is `None` is when the region is being dropped.
    parent_megabuffer: Option<Megabuffer>,
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
        if self.belongs_to_same_megabuffer(&other) {
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
            self.parent_megabuffer
                .take()
                .expect("AllocatedMegabufferRegion does not have a reference to a Megabuffer")
                .deallocate_region(self)
                .unwrap();
        }
    }
}
