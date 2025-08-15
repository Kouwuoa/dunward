use ash::vk;
use std::hash::Hash;

pub(crate) struct Queue {
    pub family: QueueFamily,
    pub handle: vk::Queue,
}

impl Queue {
    pub fn new(family: QueueFamily, handle: vk::Queue) -> Self {
        Self { family, handle }
    }
}

#[derive(Clone)]
pub(crate) struct QueueFamily {
    pub index: u32,
    pub properties: vk::QueueFamilyProperties,
    supports_present: bool,
}

impl QueueFamily {
    pub fn new(index: u32, properties: vk::QueueFamilyProperties, supports_present: bool) -> Self {
        Self {
            index,
            properties,
            supports_present,
        }
    }

    pub fn supports_present(&self) -> bool {
        self.supports_present
    }

    pub fn supports_graphics(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::GRAPHICS)
    }

    pub fn supports_compute(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::COMPUTE)
    }

    pub fn supports_transfer(&self) -> bool {
        self.properties
            .queue_flags
            .contains(vk::QueueFlags::TRANSFER)
    }

    pub fn supports_sparse_binding(&self) -> bool {
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
