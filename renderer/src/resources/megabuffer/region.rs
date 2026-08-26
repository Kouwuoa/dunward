use super::{Megabuffer, MegabufferId, MegabufferWriter, aligned_size};
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct FreeMegabufferRegion {
    pub size: u64,
    pub offset: u64,
}

pub(crate) struct AllocatedMegabufferRegion {
    /// Size of the allocated region. This is 0 when the region is deallocated.
    size: u64,
    /// Offset of the allocated region within the parent megabuffer. This is ignored when the region is deallocated.
    offset: u64,
    alignment: u64,
    megabuffer_id: MegabufferId,
    writer: Arc<MegabufferWriter>,
}

impl AllocatedMegabufferRegion {
    pub fn new(
        offset: u64,
        size: u64,
        alignment: u64,
        megabuffer_id: MegabufferId,
        writer: Arc<MegabufferWriter>,
    ) -> Self {
        Self {
            offset,
            size,
            alignment,
            megabuffer_id,
            writer,
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn write<T>(&mut self, data: &[T]) -> Result<()>
    where
        T: Copy,
    {
        self.writer.write(data, self)
    }

    pub fn suballocate_region(&mut self, size: u64) -> Result<AllocatedMegabufferRegion> {
        let size = aligned_size(size, self.alignment);

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
            size,
            offset: self.offset + (self.size - size),
            alignment: self.alignment,
            megabuffer_id: self.megabuffer_id,
            writer: self.writer.clone(),
        };
        self.size -= size;

        Ok(subregion)
    }

    pub fn belongs_to_same_megabuffer(&self, other: &Self) -> bool {
        self.megabuffer_id == other.megabuffer_id
    }

    pub fn belongs_to_megabuffer(&self, megabuffer: &Megabuffer) -> bool {
        self.megabuffer_id == megabuffer.id
    }

    pub fn belongs_to_megabuffer_id(&self, megabuffer_id: MegabufferId) -> bool {
        self.megabuffer_id == megabuffer_id
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
