//! Queue management and queue family properties for Vulkan hardware queues.
//!
//! Wraps raw [`ash::vk::Queue`] handles and tracks queue family capabilities
//! (Graphics, Compute, Transfer, Present).

use ash::vk;
use std::hash::Hash;

pub(crate) struct Queue {
    pub(crate) family: QueueFamily,
    pub(crate) handle: vk::Queue,
}

impl Queue {
    pub(crate) fn new(family: QueueFamily, handle: vk::Queue) -> Self {
        Self { family, handle }
    }
}

#[derive(Clone)]
pub(crate) struct QueueFamily {
    pub(crate) index: u32,
    pub(crate) properties: vk::QueueFamilyProperties,
    supports_present: bool,
}

impl QueueFamily {
    pub(crate) fn new(index: u32, properties: vk::QueueFamilyProperties, supports_present: bool) -> Self {
        Self {
            index,
            properties,
            supports_present,
        }
    }

    pub(crate) fn supports_present(&self) -> bool {
        self.supports_present
    }

    pub(crate) fn supports_graphics(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::GRAPHICS)
    }

    pub(crate) fn supports_compute(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::COMPUTE)
    }

    pub(crate) fn supports_transfer(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::TRANSFER)
    }

    pub(crate) fn supports_sparse_binding(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::SPARSE_BINDING)
    }
}

impl PartialEq for QueueFamily {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl Eq for QueueFamily {}

impl Hash for QueueFamily {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
    }
}
