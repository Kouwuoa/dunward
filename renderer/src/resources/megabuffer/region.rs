use super::{MegabufferId, MegabufferWriter};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct FreeMegabufferRegion {
    pub size: u64,
    pub offset: u64,
}

pub(crate) struct AllocatedMegabufferRegion {
    /// Size of the allocated region. This is 0 when the region is deallocated.
    pub size: u64,
    /// Offset of the allocated region within the parent megabuffer. This is ignored when the region is deallocated.
    pub offset: u64,
    writer: Arc<MegabufferWriter>,
}

impl AllocatedMegabufferRegion {
    pub fn new(size: u64, offset: u64, writer: Arc<MegabufferWriter>) -> Self {
        Self {
            size,
            offset,
            writer,
        }
    }

    pub fn write<T>(&mut self, data: &[T]) -> Result<()>
    where
        T: Copy,
    {
        let data_size = size_of_val(data) as u64;
        if data_size > region.size {
            return Err(eyre!("Data too large for region"));
        }

        // Allocate a temporary staging buffer ONLY for this data
        let mut staging_buffer = Buffer::new(
            data_size,
            self.alignment,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            self.mem_allocator.clone(),
            self.device.clone(),
        )?;

        let _ = staging_buffer.write(data, 0)?;

        let mut state = self.lock()?;
        state.queue_upload(staging_buffer, region.offset, data_size);
    }

    pub fn suballocate_region(&mut self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let size = self
            .parent_megabuffer_state
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

    pub fn belongs_to_megabuffer_id(&self, megabuffer_id: MegabufferId) -> bool {
        self.parent_megabuffer.as_ref().unwrap().id == megabuffer_id
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

    fn lock(&self) -> Result<MutexGuard<MegabufferState>> {
        self.parent_megabuffer_state
            .lock()
            .map_err(|e| eyre!(e.to_string()))
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
