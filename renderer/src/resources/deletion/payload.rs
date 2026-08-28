use crate::gpu::Gpu;
use ash::vk;

pub(crate) enum DeletionPayload {
    Texture {
        handle: vk::Image,
        view: Option<vk::ImageView>,
        allocation: Option<vk_mem::Allocation>,
    },
    Buffer {
        handle: vk::Buffer,
        allocation: vk_mem::Allocation,
    },
}

impl DeletionPayload {
    pub fn destroy(self, gpu: &Gpu) {
        match self {
            DeletionPayload::Texture { handle, view, allocation } => {
                if let Some(view) = view {
                    gpu.destroy_vk_image_view(view);
                }
                if let Some(allocation) = allocation {
                    gpu.destroy_vk_image(handle, allocation);
                }
            }
            DeletionPayload::Buffer { handle, allocation } => {
                gpu.destroy_vk_buffer(handle, allocation);
            }
        }
    }
}

