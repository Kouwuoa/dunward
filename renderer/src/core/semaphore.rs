//! Binary and timeline semaphore wrappers for GPU synchronization.
//!
//! Provides type-safe wrappers for Vulkan semaphores, supporting both
//! binary semaphores (for swapchain acquire/present) and timeline semaphores
//! (for fine-grained multi-queue cross-frame timeline tracking).

use ash::vk;

#[repr(transparent)]
pub(crate) struct BinarySemaphore(vk::Semaphore);

#[repr(transparent)]
pub(crate) struct TimelineSemaphore(vk::Semaphore);

#[derive(Debug, Clone, Copy)]
pub(crate) enum SemaphoreValue {
    Binary,
    Timeline(u64),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WaitSemaphore {
    pub semaphore: vk::Semaphore,
    pub stage_mask: vk::PipelineStageFlags,
    pub value: SemaphoreValue,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SignalSemaphore {
    pub semaphore: vk::Semaphore,
    pub value: SemaphoreValue,
}

impl BinarySemaphore {
    pub fn new(sem: vk::Semaphore) -> Self {
        Self(sem)
    }

    pub fn raw(&self) -> vk::Semaphore {
        self.0
    }

    pub fn to_wait_semaphore(&self, stage_mask: vk::PipelineStageFlags) -> WaitSemaphore {
        WaitSemaphore {
            semaphore: self.0,
            stage_mask,
            value: SemaphoreValue::Binary,
        }
    }

    pub fn to_signal_semaphore(&self) -> SignalSemaphore {
        SignalSemaphore {
            semaphore: self.0,
            value: SemaphoreValue::Binary,
        }
    }
}

impl TimelineSemaphore {
    pub fn new(sem: vk::Semaphore) -> Self {
        Self(sem)
    }

    pub fn raw(&self) -> vk::Semaphore {
        self.0
    }

    pub fn to_wait_semaphore(
        &self,
        stage_mask: vk::PipelineStageFlags,
        value: u64,
    ) -> WaitSemaphore {
        WaitSemaphore {
            semaphore: self.0,
            stage_mask,
            value: SemaphoreValue::Timeline(value),
        }
    }

    pub fn to_signal_semaphore(&self, value: u64) -> SignalSemaphore {
        SignalSemaphore {
            semaphore: self.0,
            value: SemaphoreValue::Timeline(value),
        }
    }
}
