//! Command recorder allocation and per-queue command pool management.
//!
//! Exposes [`CommandRecorderAllocator`], managing a [`ash::vk::CommandPool`] for each
//! queue family and allocating recyclable [`CommandRecorder`] instances.

use std::collections::{HashMap, hash_map};
use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use super::recorder::{CommandRecorder, Idle};
use crate::core::queue::{Queue, QueueFamily};

#[repr(transparent)]
pub(crate) struct CommandRecorderAllocator(Arc<Mutex<CommandRecorderAllocatorInner>>);

impl Clone for CommandRecorderAllocator {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

pub(crate) trait CommandRecorderAllocatorExt<A> {
    fn new(device: Arc<ash::Device>) -> Result<A>;
    /// Note that this is mutably borrowed to force the allocator to be used in a single-threaded context.
    fn allocate(&mut self, queue: Arc<Queue>) -> Result<CommandRecorder<Idle>>;
    /// Note that this is mutably borrowed to force the allocator to be used in a single-threaded context.
    fn deallocate(&mut self, command_encoder: CommandRecorder<Idle>) -> Result<()>;
}

struct CommandRecorderAllocatorInner {
    command_pools: HashMap<QueueFamily, vk::CommandPool>,
    allocated_command_buffers: HashMap<QueueFamily, Vec<vk::CommandBuffer>>,
    device: Arc<ash::Device>,
}

impl CommandRecorderAllocatorExt<CommandRecorderAllocator> for CommandRecorderAllocator {
    fn new(device: Arc<ash::Device>) -> Result<CommandRecorderAllocator> {
        Ok(CommandRecorderAllocator(Arc::new(Mutex::new(
            CommandRecorderAllocatorInner {
                command_pools: HashMap::new(),
                allocated_command_buffers: HashMap::new(),
                device,
            },
        ))))
    }

    fn allocate(&mut self, queue: Arc<Queue>) -> Result<CommandRecorder<Idle>> {
        let (command_buffer, device) = {
            let mut guard = self.0.lock().map_err(|e| eyre!(e.to_string()))?;

            let device = guard.device.clone();

            let command_pool = match guard.command_pools.entry(queue.family.clone()) {
                hash_map::Entry::Vacant(entry) => {
                    let pool_info = vk::CommandPoolCreateInfo::default()
                        .queue_family_index(queue.family.index)
                        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
                    let pool = unsafe { device.create_command_pool(&pool_info, None)? };
                    entry.insert(pool)
                }
                hash_map::Entry::Occupied(entry) => entry.into_mut(),
            };

            let command_buffer_info = vk::CommandBufferAllocateInfo::default()
                .command_pool(*command_pool)
                .command_buffer_count(1)
                .level(vk::CommandBufferLevel::PRIMARY);
            let command_buffer = unsafe {
                guard
                    .device
                    .allocate_command_buffers(&command_buffer_info)?[0]
            };

            guard
                .allocated_command_buffers
                .entry(queue.family.clone())
                .or_insert_with(Vec::new)
                .push(command_buffer);

            (command_buffer, device)
        };

        let command_encoder = CommandRecorder::new(command_buffer, queue, device, self.clone());

        Ok(command_encoder)
    }

    fn deallocate(&mut self, command_recorder: CommandRecorder<Idle>) -> Result<()> {
        let mut guard = self.0.lock().map_err(|e| eyre!(e.to_string()))?;

        let command_pool = guard
            .command_pools
            .get(&command_recorder.get_queue().family)
            .ok_or_eyre(format!(
                "Failed to get command pool for queue family: {}",
                command_recorder.get_queue().family.index
            ))?;
        let command_buffer = command_recorder.get_command_buffer();
        unsafe {
            guard
                .device
                .free_command_buffers(*command_pool, &[command_buffer]);
        }
        let command_buffers = guard
            .allocated_command_buffers
            .get_mut(&command_recorder.get_queue().family)
            .ok_or_eyre(format!(
                "Failed to get command buffers for queue family: {}",
                command_recorder.get_queue().family.index
            ))?;
        let index = command_buffers
            .iter()
            .position(|&cb| cb == command_buffer)
            .ok_or_eyre(format!(
                "Failed to find command buffer in vec for queue family: {}",
                command_recorder.get_queue().family.index
            ))?;
        let _ = command_buffers.swap_remove(index);
        Ok(())
    }
}

impl Drop for CommandRecorderAllocatorInner {
    fn drop(&mut self) {
        let command_pools = self.command_pools.drain().collect::<Vec<_>>();

        for (queue_family, command_pool) in command_pools {
            let command_buffers = self
                .allocated_command_buffers
                .remove(&queue_family)
                .unwrap();
            unsafe {
                if !command_buffers.is_empty() {
                    self.device
                        .free_command_buffers(command_pool, &command_buffers);
                }
                self.device.destroy_command_pool(command_pool, None);
            }
        }
    }
}
