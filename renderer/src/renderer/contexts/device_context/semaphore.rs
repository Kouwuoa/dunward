use ash::vk;

#[derive(Debug, Clone, Copy)]
pub(crate) enum SemaphoreValue {
    Binary,
    Timeline(u64),
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct WaitSemaphore {
    pub(super) semaphore: vk::Semaphore,
    pub(super) stage_mask: vk::PipelineStageFlags,
    pub(super) value: SemaphoreValue,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SignalSemaphore {
    pub(super) semaphore: vk::Semaphore,
    pub(super) value: SemaphoreValue,
}
