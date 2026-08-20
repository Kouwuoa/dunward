//! Pooled Vulkan descriptor allocator.
//!
//! Manages multiple pooled [`ash::vk::DescriptorPool`] instances, automatically
//! allocating new pools when full and reusing pools across frames.

use std::sync::Arc;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;

#[derive(Debug, Clone)]
struct DescriptorAllocatorPoolSizeRatio {
    desc_type: vk::DescriptorType,
    ratio: f32,
}

pub(crate) struct DescriptorAllocator {
    pool_ratios: Vec<DescriptorAllocatorPoolSizeRatio>, // Needed to reallocate pools
    full_pools: Vec<vk::DescriptorPool>,                // Pools that cannot allocate more sets
    ready_pools: Vec<vk::DescriptorPool>,               // Pools that can allocate more sets
    sets_per_pool: u32,
    device: Arc<ash::Device>,
}

impl Drop for DescriptorAllocator {
    fn drop(&mut self) {
        self.destroy_pools();
    }
}

impl DescriptorAllocator {
    pub fn new(device: Arc<ash::Device>, initial_sets_per_pool: u32) -> Result<Self> {
        let pool_ratios = [
            DescriptorAllocatorPoolSizeRatio {
                desc_type: vk::DescriptorType::STORAGE_IMAGE,
                ratio: 3.0,
            },
            DescriptorAllocatorPoolSizeRatio {
                desc_type: vk::DescriptorType::STORAGE_BUFFER,
                ratio: 3.0,
            },
            DescriptorAllocatorPoolSizeRatio {
                desc_type: vk::DescriptorType::UNIFORM_BUFFER,
                ratio: 3.0,
            },
            DescriptorAllocatorPoolSizeRatio {
                desc_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                ratio: 4.0,
            },
        ];

        // Allocate the first descriptor pool and add it to ready_pools
        let new_pool = Self::create_pool(&device, initial_sets_per_pool, &pool_ratios)?;
        let ready_pools = vec![new_pool];

        Ok(Self {
            pool_ratios: pool_ratios.to_vec(),
            full_pools: Vec::new(),
            ready_pools,
            sets_per_pool: initial_sets_per_pool,
            device,
        })
    }

    pub fn allocate(&mut self, set_layout: vk::DescriptorSetLayout) -> Result<vk::DescriptorSet> {
        let set_layouts = [set_layout];
        let mut pool_to_use = self.get_or_create_pool()?;

        let mut alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool: pool_to_use,
            descriptor_set_count: 1,
            p_set_layouts: set_layouts.as_ptr(),
            ..Default::default()
        };

        let desc_set = match unsafe { self.device.allocate_descriptor_sets(&alloc_info) } {
            Ok(desc_set) => Ok(desc_set[0]),
            Err(err) => {
                // If the pool is full, push the pool to full_pools and create a new pool
                if err == vk::Result::ERROR_OUT_OF_POOL_MEMORY
                    || err == vk::Result::ERROR_FRAGMENTED_POOL
                {
                    self.full_pools.push(pool_to_use);
                    pool_to_use = self.get_or_create_pool()?;
                    alloc_info.descriptor_pool = pool_to_use;
                    // If getting a new pool fails, don't try again because stuff is broken
                    Ok(unsafe { self.device.allocate_descriptor_sets(&alloc_info)?[0] })
                } else {
                    Err(eyre!("Failed to allocate descriptor set: {:?}", err))
                }
            }
        }?;
        self.ready_pools.push(pool_to_use);

        Ok(desc_set)
    }

    /// Reset all pools and mark all "full" pools as "ready" pools
    pub fn reset_pools(&mut self) -> Result<()> {
        for pool in self.ready_pools.iter() {
            unsafe {
                self.device
                    .reset_descriptor_pool(*pool, vk::DescriptorPoolResetFlags::empty())?;
            }
        }

        for pool in self.full_pools.drain(..) {
            unsafe {
                self.device
                    .reset_descriptor_pool(pool, vk::DescriptorPoolResetFlags::empty())?;
                self.ready_pools.push(pool);
            }
        }

        Ok(())
    }

    /// Destroy all pools currently managed by this allocator. A new pool will need to be created the next time `allocate()` gets called.
    pub fn destroy_pools(&mut self) {
        for pool in self.ready_pools.drain(..) {
            unsafe {
                self.device.destroy_descriptor_pool(pool, None);
            }
        }

        for pool in self.full_pools.drain(..) {
            unsafe {
                self.device.destroy_descriptor_pool(pool, None);
            }
        }
    }

    fn get_or_create_pool(&mut self) -> Result<vk::DescriptorPool> {
        if let Some(ready_pool) = self.ready_pools.pop() {
            Ok(ready_pool)
        } else {
            // Ran out of pools

            // Increase number of sets per pool by 50% for the next pool allocation
            let sets_per_pool = (self.sets_per_pool as f32 * 1.5) as u32;
            self.sets_per_pool = sets_per_pool.min(4092); // Limit max sets per pool

            // Create a new pool
            Self::create_pool(&self.device, self.sets_per_pool, &self.pool_ratios)
        }
    }

    fn create_pool(
        device: &ash::Device,
        set_count: u32,
        ratios: &[DescriptorAllocatorPoolSizeRatio],
    ) -> Result<vk::DescriptorPool> {
        let pool_sizes = ratios
            .iter()
            .map(|ratio| vk::DescriptorPoolSize {
                ty: ratio.desc_type,
                descriptor_count: (ratio.ratio * set_count as f32) as u32,
            })
            .collect::<Vec<vk::DescriptorPoolSize>>();

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(set_count)
            .pool_sizes(&pool_sizes)
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND);

        Ok(unsafe { device.create_descriptor_pool(&pool_info, None)? })
    }
}
