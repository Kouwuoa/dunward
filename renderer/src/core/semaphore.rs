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
    pub(crate) semaphore: vk::Semaphore,
    pub(crate) stage_mask: vk::PipelineStageFlags,
    pub(crate) value: SemaphoreValue,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SignalSemaphore {
    pub(crate) semaphore: vk::Semaphore,
    pub(crate) value: SemaphoreValue,
}

impl BinarySemaphore {
    pub(crate) fn new(sem: vk::Semaphore) -> Self {
        Self(sem)
    }

    pub(crate) fn raw(&self) -> vk::Semaphore {
        self.0
    }

    pub(crate) fn to_wait_semaphore(&self, stage_mask: vk::PipelineStageFlags) -> WaitSemaphore {
        WaitSemaphore {
            semaphore: self.0,
            stage_mask,
            value: SemaphoreValue::Binary,
        }
    }

    pub(crate) fn to_signal_semaphore(&self) -> SignalSemaphore {
        SignalSemaphore {
            semaphore: self.0,
            value: SemaphoreValue::Binary,
        }
    }
}

impl TimelineSemaphore {
    pub(crate) fn new(sem: vk::Semaphore) -> Self {
        Self(sem)
    }

    pub(crate) fn raw(&self) -> vk::Semaphore {
        self.0
    }

    pub(crate) fn to_wait_semaphore(
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

    pub(crate) fn to_signal_semaphore(&self, value: u64) -> SignalSemaphore {
        SignalSemaphore {
            semaphore: self.0,
            value: SemaphoreValue::Timeline(value),
        }
    }
}
