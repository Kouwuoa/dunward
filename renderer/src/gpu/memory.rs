//! GPU memory allocation and deallocation methods for [`Gpu`]

use crate::gpu::Gpu;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use vk_mem::Alloc;

impl Gpu {
    pub fn allocate_vk_image(
        &self,
        image_info: &vk::ImageCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
    ) -> Result<(vk::Image, vk_mem::Allocation)> {
        unsafe {
            self.mem_allocator
                .lock()
                .map_err(|e| eyre!(e.to_string()))?
                .create_image(&image_info, &allocation_info)
                .map_err(|e| eyre!(e.to_string()))
        }
    }

    pub fn destroy_vk_image(&self, image: vk::Image, mut allocation: vk_mem::Allocation) {
        unsafe {
            self.mem_allocator
                .lock()
                .expect("Failed to acquire lock for memory allocator")
                .destroy_image(image, &mut allocation);
        }
    }

    pub fn allocate_vk_buffer(
        &self,
        buffer_info: &vk::BufferCreateInfo,
        allocation_info: &vk_mem::AllocationCreateInfo,
        alignment: vk::DeviceSize,
    ) -> Result<(vk::Buffer, vk_mem::Allocation)> {
        unsafe {
            self.mem_allocator
                .lock()
                .map_err(|e| eyre!(e.to_string()))?
                .create_buffer_with_alignment(&buffer_info, &allocation_info, alignment)
                .map_err(|e| eyre!(e.to_string()))
        }
    }

    pub fn destroy_vk_buffer(&self, buffer: vk::Buffer, mut allocation: vk_mem::Allocation) {
        unsafe {
            self.mem_allocator
                .lock()
                .expect("Failed to acquire lock for memory allocator")
                .destroy_buffer(buffer, &mut allocation);
        }
    }

    pub fn get_allocation_info(
        &self,
        allocation: &vk_mem::Allocation,
    ) -> Result<vk_mem::AllocationInfo> {
        Ok(self
            .mem_allocator
            .lock()
            .map_err(|e| eyre!(e.to_string()))?
            .get_allocation_info(allocation))
    }
}
