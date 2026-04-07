use ash::vk;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Semaphore {
    pub(super) semaphore: vk::Semaphore,
    pub(super) wait_stage_mask: vk::PipelineStageFlags,
    /// For timeline semaphores, the value to wait/signal.
    /// For binary semaphores, this is ignored.
    pub(super) value: Option<u64>,
}
