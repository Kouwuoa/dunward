//! Low-level GPU memory buffer wrapper.
//!
//! Wraps [`ash::vk::Buffer`] and [`vk_mem::Allocation`], supporting host-visible
//! memory-mapped writes via [`presser`].

use crate::gpu::Gpu;

use ash::vk;
use color_eyre::eyre::{Result, eyre};
use std::sync::Arc;

pub(crate) struct Buffer {
    buffer: vk::Buffer,
    size: u64,
    alignment: u64,
    mapped: bool,

    allocation: Option<vk_mem::Allocation>,
    gpu: Arc<Gpu>,
}

impl Buffer {
    pub fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
        mem_usage: vk_mem::MemoryUsage,
        mapped: bool,
        gpu: Arc<Gpu>,
    ) -> Result<Self> {
        let (buffer, allocation) = {
            let buffer_info = vk::BufferCreateInfo {
                size,
                usage: buf_usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: mem_usage,
                flags: if mapped {
                    vk_mem::AllocationCreateFlags::MAPPED
                        | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                } else {
                    vk_mem::AllocationCreateFlags::empty()
                },
                ..Default::default()
            };
            gpu.allocate_vk_buffer(&buffer_info, &allocation_info, alignment)?
        };

        Ok(Self {
            buffer,
            size,
            alignment,
            mapped,
            allocation: Some(allocation),
            gpu,
        })
    }

    pub fn write<T>(&mut self, data: &[T], start_offset: usize) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if !self.mapped {
            return Err(eyre!("Cannot write to buffer that is not mapped"));
        }

        let allocation = self.allocation.as_ref().expect("Allocation does not exist");
        let allocation_info = self.gpu.get_allocation_info(allocation)?;

        if size_of_val(data) as u64 > allocation_info.size {
            return Err(eyre!("Data too large to write into buffer"));
        }

        let mut raw_allocation = presser::RawAllocation::from_raw_parts(
            std::ptr::NonNull::new(allocation_info.mapped_data as *mut u8)
                .expect("Mapped data pointer was null"),
            allocation_info.size as usize,
        );
        let mut slab = unsafe { raw_allocation.borrow_as_slab() };
        let copy_record = presser::copy_to_offset(&data, &mut slab, start_offset)?;

        Ok(copy_record)
    }

    pub fn raw(&self) -> vk::Buffer {
        self.buffer
    }

    pub fn alignment(&self) -> u64 {
        self.alignment
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let allocation = self.allocation.as_mut().expect("Allocation does not exist");
        self.gpu.destroy_vk_buffer(self.buffer, allocation);
    }
}
