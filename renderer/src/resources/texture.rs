//! GPU Texture wrappers, layouts, memory allocation, and access tracking.
//!
//! Exposes [`Texture`], [`ColorTexture`], [`DepthTexture`], and [`StorageTexture`],
//! tracking image formats, layouts, stage/access masks ([`TextureAccess`]),
//! and queue family ownership ([`TextureQueueState`]).

use super::buffer::Buffer;
use crate::commands::transfer::TransferCommandRecorder;
use crate::gpu::Gpu;
use crate::gpu::queue::Queue;

use ash::vk;
use color_eyre::eyre::Result;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[repr(transparent)]
pub(crate) struct ColorTexture(pub Texture);
impl Deref for ColorTexture {
    type Target = Texture;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for ColorTexture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[repr(transparent)]
pub(crate) struct DepthTexture(pub Texture);
impl Deref for DepthTexture {
    type Target = Texture;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for DepthTexture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[repr(transparent)]
pub(crate) struct StorageTexture(pub Texture);
impl Deref for StorageTexture {
    type Target = Texture;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl DerefMut for StorageTexture {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

struct TextureCreateInfo {
    format: vk::Format,
    extent: vk::Extent3D,
    usage: vk::ImageUsageFlags,
    aspect: vk::ImageAspectFlags,
    /// Should be true for larger images like fullscreen images
    use_dedicated_memory: bool,
}

pub(crate) struct Texture {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub aspect: vk::ImageAspectFlags,
    pub layout: vk::ImageLayout,
    pub access_state: TextureAccess,
    pub queue_state: TextureQueueState,

    /// Determines if the dtor should destroy the vk::ImageView associated with this texture.
    /// If false, this `Texture` will NOT responsible for the lifetime of the `vk::ImageView`.
    destroy_view: bool,

    allocation: Option<vk_mem::Allocation>, // GPU-only memory block
    gpu: Arc<Gpu>,
}

pub(crate) enum TextureQueueState {
    /// Texture is exclusively owned and usable by `queue`
    Owned { queue: Arc<Queue> },
    /// Released by `src_queue` and pending acquire by `dst_queue`
    Transferring {
        src_queue: Arc<Queue>,
        dst_queue: Arc<Queue>,
    },
    /// Freshly created texture; not yet used by any queue
    /// The first queue that records a barrier will claim ownership
    Uninitialized,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextureAccess {
    pub stage_mask: vk::PipelineStageFlags2,
    pub access_mask: vk::AccessFlags2,
}

impl Texture {
    /// NOTE: The `allocation` field of the Image this function returns is GPU-only
    /// and is NOT yet populated with any data.
    /// This means that unless you are making a depth image or storage image, you will need to call
    /// `upload()`
    fn new(create_info: &TextureCreateInfo, gpu: Arc<Gpu>) -> Result<Texture> {
        let (image, allocation) = {
            let image_info = vk::ImageCreateInfo::default()
                .format(create_info.format)
                .usage(create_info.usage)
                .extent(create_info.extent)
                .image_type(vk::ImageType::TYPE_2D)
                .mip_levels(1)
                .array_layers(1)
                .samples(vk::SampleCountFlags::TYPE_1)
                .tiling(vk::ImageTiling::OPTIMAL);
            let allocation_info = vk_mem::AllocationCreateInfo {
                usage: vk_mem::MemoryUsage::AutoPreferDevice,
                flags: if create_info.use_dedicated_memory {
                    vk_mem::AllocationCreateFlags::DEDICATED_MEMORY
                } else {
                    vk_mem::AllocationCreateFlags::empty()
                },
                ..Default::default()
            };
            gpu.allocate_vk_image(&image_info, &allocation_info)?
        };

        let view = {
            let info = vk::ImageViewCreateInfo::default()
                .view_type(vk::ImageViewType::TYPE_2D)
                .image(image)
                .format(create_info.format)
                .subresource_range(vk::ImageSubresourceRange {
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                    aspect_mask: create_info.aspect,
                });
            gpu.create_vk_image_view(&info)?
        };

        Ok(Self {
            image,
            view,
            format: create_info.format,
            extent: create_info.extent,
            aspect: create_info.aspect,
            layout: vk::ImageLayout::UNDEFINED,
            access_state: TextureAccess {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_mask: vk::AccessFlags2::NONE,
            },
            queue_state: TextureQueueState::Uninitialized,

            destroy_view: true, // Since we created the view in this ctor, we'll need to clean it up

            allocation: Some(allocation),
            gpu,
        })
    }

    /// Create a 32-bit shader-readable texture from a byte array
    pub fn new_color_texture_from_bytes(
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
        usage: vk::ImageUsageFlags,
        gpu: Arc<Gpu>,
        transfer: &mut TransferCommandRecorder,
    ) -> Result<ColorTexture> {
        let image = {
            let create_info = TextureCreateInfo {
                format: vk::Format::R8G8B8A8_SRGB,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                usage,
                aspect: vk::ImageAspectFlags::COLOR,
                use_dedicated_memory,
            };
            let mut image = Self::new(&create_info, gpu)?;

            if let Some(data) = data {
                image.upload(data, transfer)?;
            }

            image
        };

        Ok(ColorTexture(image))
    }

    pub fn new_color_texture_from_image(
        image: &image::DynamicImage,
        use_dedicated_memory: bool,
        usage: vk::ImageUsageFlags,
        gpu: Arc<Gpu>,
        transfer: &mut TransferCommandRecorder,
    ) -> Result<ColorTexture> {
        let data = image.to_rgba8().into_raw();
        let width = image.width();
        let height = image.height();
        Self::new_color_texture_from_bytes(
            width,
            height,
            Some(&data),
            use_dedicated_memory,
            usage,
            gpu,
            transfer,
        )
    }

    /// # Arguments
    /// * `destroy_view` - If false, this function creates a `ColorTexture` that is NOT responsible for the lifetime of the `vk::ImageView`
    pub fn new_color_texture_from_vkimage(
        image: &vk::Image,
        view: &vk::ImageView,
        format: &vk::Format,
        extent: &vk::Extent2D,
        destroy_view: bool,
        queue: Arc<Queue>,
        gpu: Arc<Gpu>,
    ) -> ColorTexture {
        ColorTexture(Texture {
            image: *image,
            view: *view,
            format: *format,
            extent: vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            },
            aspect: vk::ImageAspectFlags::COLOR,
            layout: vk::ImageLayout::UNDEFINED,
            access_state: TextureAccess {
                stage_mask: vk::PipelineStageFlags2::NONE,
                access_mask: vk::AccessFlags2::NONE,
            },
            queue_state: TextureQueueState::Owned { queue },
            destroy_view,
            allocation: None,
            gpu,
        })
    }

    /// Create a special type of texture used for the depth buffer
    pub fn new_depth_texture(width: u32, height: u32, gpu: Arc<Gpu>) -> Result<DepthTexture> {
        let create_info = TextureCreateInfo {
            format: vk::Format::D32_SFLOAT,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            aspect: vk::ImageAspectFlags::DEPTH,
            use_dedicated_memory: true, // Assuming the depth image will be used as a fullscreen attachment
        };
        Ok(DepthTexture(Self::new(&create_info, gpu)?))
    }

    /// Create a special type of texture likely used by compute shaders
    pub fn new_storage_texture(
        width: u32,
        height: u32,
        use_dedicated_memory: bool,
        gpu: Arc<Gpu>,
    ) -> Result<StorageTexture> {
        let image = {
            let extent = vk::Extent3D {
                width,
                height,
                depth: 1,
            };
            let usage = vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE;
            let create_info = TextureCreateInfo {
                format: vk::Format::R16G16B16A16_SFLOAT,
                extent,
                usage,
                aspect: vk::ImageAspectFlags::COLOR,
                use_dedicated_memory,
            };
            Texture::new(&create_info, gpu)?
        };

        Ok(StorageTexture(image))
    }

    pub fn width(&self) -> u32 {
        self.extent.width
    }

    pub fn height(&self) -> u32 {
        self.extent.height
    }

    pub fn blit_to_vkimage(
        &self,
        dst_image: vk::Image,
        dst_image_extent: vk::Extent2D,
        cmd: vk::CommandBuffer,
    ) {
        blit_vkimage_to_vkimage(
            cmd,
            self.image,
            dst_image,
            vk::Extent2D {
                width: self.extent.width,
                height: self.extent.height,
            },
            dst_image_extent,
            &self.gpu.raw_logical(),
        );
    }

    pub fn blit_to(&mut self, dst: &mut Texture, cmd: vk::CommandBuffer) {
        self.blit_to_vkimage(
            dst.image,
            vk::Extent2D {
                width: dst.extent.width,
                height: dst.extent.height,
            },
            cmd,
        );

        self.access_state.stage_mask |= vk::PipelineStageFlags2::TRANSFER;
        self.access_state.access_mask |= vk::AccessFlags2::TRANSFER_READ;
        dst.access_state = TextureAccess {
            stage_mask: vk::PipelineStageFlags2::TRANSFER,
            access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        };
    }

    fn upload(&mut self, data: &[u8], transfer: &mut TransferCommandRecorder) -> Result<()> {
        let mut staging_buffer = Buffer::new(
            data.len() as u64,
            256,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            self.gpu.clone(),
        )?;
        staging_buffer.write(data, 0)?;
        transfer.immediate_submit(|cmd: vk::CommandBuffer, _device: &ash::Device| {
            let range = vk::ImageSubresourceRange {
                aspect_mask: self.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            };

            let img_barrier_to_transfer = vk::ImageMemoryBarrier {
                old_layout: vk::ImageLayout::UNDEFINED,
                new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                image: self.image,
                subresource_range: range,
                src_access_mask: vk::AccessFlags::empty(),
                dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
                ..Default::default()
            };

            unsafe {
                self.gpu.raw_logical().cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[img_barrier_to_transfer],
                );
            }

            let copy_region = vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: self.aspect,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                image_extent: self.extent,
                ..Default::default()
            };

            unsafe {
                self.gpu.raw_logical().cmd_copy_buffer_to_image(
                    cmd,
                    staging_buffer.raw(),
                    self.image,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    &[copy_region],
                );
            }

            let mut img_barrier_to_readable = img_barrier_to_transfer;
            img_barrier_to_readable.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            img_barrier_to_readable.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            img_barrier_to_readable.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            img_barrier_to_readable.dst_access_mask = vk::AccessFlags::SHADER_READ;

            unsafe {
                self.gpu.raw_logical().cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::COMPUTE_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[img_barrier_to_readable],
                )
            }

            Ok(())
        })?;

        Ok(())
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        if self.destroy_view {
            self.gpu.destroy_vk_image_view(self.view);
        }
        if let Some(allocation) = self.allocation.as_mut() {
            self.gpu.destroy_vk_image(self.image, allocation);
        }
    }
}

fn blit_vkimage_to_vkimage(
    cmd: vk::CommandBuffer,
    src: vk::Image,
    dst: vk::Image,
    src_size: vk::Extent2D,
    dst_size: vk::Extent2D,
    device: &ash::Device,
) {
    let blit_region = vk::ImageBlit2 {
        src_offsets: [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: src_size.width as i32,
                y: src_size.height as i32,
                z: 1,
            },
        ],
        dst_offsets: [
            vk::Offset3D { x: 0, y: 0, z: 0 },
            vk::Offset3D {
                x: dst_size.width as i32,
                y: dst_size.height as i32,
                z: 1,
            },
        ],
        src_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            layer_count: 1,
            mip_level: 0,
        },
        dst_subresource: vk::ImageSubresourceLayers {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_array_layer: 0,
            layer_count: 1,
            mip_level: 0,
        },
        ..Default::default()
    };

    let blit_info = vk::BlitImageInfo2 {
        dst_image: dst,
        dst_image_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        src_image: src,
        src_image_layout: vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        filter: vk::Filter::LINEAR,
        region_count: 1,
        p_regions: &blit_region,
        ..Default::default()
    };

    unsafe {
        device.cmd_blit_image2(cmd, &blit_info);
    }
}
