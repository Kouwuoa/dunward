use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use vk_mem::Alloc;

pub(crate) struct Buffer {
    pub buffer: vk::Buffer,
    pub size: u64,
    mapped: bool,

    allocation: Option<vk_mem::Allocation>,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    device: Arc<ash::Device>,
}

impl Buffer {
    pub fn new(
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
        mem_usage: vk_mem::MemoryUsage,
        mapped: bool,

        mem_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Self> {
        let (buffer, allocation) = unsafe {
            let buffer_info = vk::BufferCreateInfo {
                size,
                usage: buf_usage,
                sharing_mode: vk::SharingMode::EXCLUSIVE,
                ..Default::default()
            };
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: mem_usage,
                flags: if mapped {
                    vk_mem::AllocationCreateFlags::MAPPED | vk_mem::AllocationCreateFlags::HOST_ACCESS_SEQUENTIAL_WRITE
                } else {
                    vk_mem::AllocationCreateFlags::empty()
                },
                ..Default::default()
            };
            mem_allocator
                .lock()
                .map_err(|e| eyre!(e.to_string()))?
                .create_buffer_with_alignment(
                    &buffer_info,
                    &allocation_info,
                    alignment,
                )?
        };

        Ok(Self {
            buffer,
            size,
            mapped,

            allocation: Some(allocation),
            memory_allocator: mem_allocator,
            device,
        })
    }

    pub fn write<T>(
        &mut self,
        data: &[T],
        start_offset: usize,
    ) -> Result<presser::CopyRecord>
    where
        T: Copy,
    {
        if !self.mapped {
            return Err(eyre!("Cannot write to buffer that is not mapped"));
        }

        let allocation = self.allocation
            .as_ref()
            .expect("Allocation does not exist");

        let allocation_info = self.memory_allocator
            .lock()
            .map_err(|e| eyre!(e.to_string()))?
            .get_allocation_info(allocation);

        if std::mem::size_of_val(data) as u64 > allocation_info.size {
            return Err(eyre!("Data too large to write into buffer"));
        }

        let mut raw_allocation = presser::RawAllocation::from_raw_parts(
            std::ptr::NonNull::new(allocation_info.mapped_data as *mut u8)
                .expect("Mapped data pointer was null"),
            allocation_info.size as usize,
        );
        let mut slab = unsafe { raw_allocation.borrow_as_slab() };
        let copy_record = presser::copy_to_offset(
            &data,
            &mut slab,
            start_offset,
        )?;

        Ok(copy_record)
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            let allocation = self.allocation
                .as_mut()
                .expect("Allocation does not exist");
            self.memory_allocator
                .lock()
                .expect("Failed to acquire lock for memory allocator")
                .destroy_buffer(self.buffer, allocation);
        }
    }
}