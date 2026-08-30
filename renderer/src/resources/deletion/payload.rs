use crate::gpu::Gpu;
use ash::vk;

pub(crate) struct TextureDeletionPayload {
    pub handle: vk::Image,
    pub view: Option<vk::ImageView>,
    pub allocation: Option<vk_mem::Allocation>,
}

pub(crate) struct BufferDeletionPayload {
    pub handle: vk::Buffer,
    pub allocation: vk_mem::Allocation,
}

pub(crate) enum DeletionPayload {
    Texture(TextureDeletionPayload),
    Buffer(BufferDeletionPayload),
}

impl From<TextureDeletionPayload> for DeletionPayload {
    fn from(payload: TextureDeletionPayload) -> Self {
        DeletionPayload::Texture(payload)
    }
}

impl From<BufferDeletionPayload> for DeletionPayload {
    fn from(payload: BufferDeletionPayload) -> Self {
        DeletionPayload::Buffer(payload)
    }
}

impl DeletionPayload {
    pub fn destroy(self, gpu: &Gpu) {
        match self {
            DeletionPayload::Texture(t) => {
                if let Some(view) = t.view {
                    gpu.destroy_vk_image_view(view);
                }
                if let Some(allocation) = t.allocation {
                    gpu.destroy_vk_image(t.handle, allocation);
                }
            }
            DeletionPayload::Buffer(b) => {
                gpu.destroy_vk_buffer(b.handle, b.allocation);
            }
        }
    }
}

