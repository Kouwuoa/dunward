use super::buffer::Buffer;
use crate::context::commands::TransferCommandEncoder;
use ash::vk;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;
use std::sync::{Arc, Mutex};
use vk_mem::Alloc;

pub(crate) struct TextureCreateInfo {
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub usage: vk::ImageUsageFlags,
    pub aspect: vk::ImageAspectFlags,
    /// Should be true for larger images like fullscreen images
    pub use_dedicated_memory: bool,
}

pub(crate) type ColorTexture = Texture;
pub(crate) type DepthTexture = Texture;
pub(crate) type StorageTexture = Texture;

pub(crate) struct Texture {
    pub image: vk::Image,
    pub view: vk::ImageView,
    pub format: vk::Format,
    pub extent: vk::Extent3D,
    pub aspect: vk::ImageAspectFlags,

    allocation: Option<vk_mem::Allocation>, // GPU-only memory block
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    device: Arc<ash::Device>,
}

impl Texture {
    /// NOTE: The `allocation` field of the Image this function returns is GPU-only
    /// and is NOT yet populated with any data.
    /// This means that unless you are making a depth image or storage image, you will need to call
    /// `upload()`
    fn new(
        create_info: &TextureCreateInfo,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<Texture> {
        let (image, allocation) = unsafe {
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
            memory_allocator
                .lock()
                .map_err(|e| eyre!(e.to_string()))?
                .create_image(&image_info, &allocation_info)?
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
            unsafe { device.create_image_view(&info, None)? }
        };

        Ok(Self {
            image,
            view,
            format: create_info.format,
            extent: create_info.extent,
            aspect: create_info.aspect,

            allocation: Some(allocation),
            memory_allocator,
            device,
        })
    }

    /// Create a 32-bit shader-readable texture from a byte array
    pub fn new_color_texture_from_bytes(
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer: &TransferCommandEncoder,
    ) -> Result<ColorTexture> {
        let image = {
            let create_info = TextureCreateInfo {
                format: vk::Format::R8G8B8A8_SRGB,
                extent: vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                },
                usage: vk::ImageUsageFlags::SAMPLED
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
                aspect: vk::ImageAspectFlags::COLOR,
                use_dedicated_memory,
            };
            let mut image = Self::new(&create_info, memory_allocator, device)?;

            if let Some(data) = data {
                image.upload(data, transfer)?;
            }

            image
        };

        Ok(image)
    }

    pub fn new_color_texture_from_image(
        image: &image::DynamicImage,
        use_dedicated_memory: bool,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
        transfer: &TransferCommandEncoder,
    ) -> Result<Self> {
        let data = image.to_rgba8().into_raw();
        let width = image.width();
        let height = image.height();
        Self::new_color_texture_from_bytes(
            width,
            height,
            Some(&data),
            use_dedicated_memory,
            memory_allocator,
            device,
            transfer,
        )
    }

    /// Create a special type of texture used for the depth buffer
    pub fn new_depth_texture(
        width: u32,
        height: u32,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
    ) -> Result<DepthTexture> {
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
        Self::new(&create_info, memory_allocator, device)
    }

    /// Create a special type of texture likely used by compute shaders
    pub fn new_storage_texture(
        width: u32,
        height: u32,
        use_dedicated_memory: bool,

        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
        device: Arc<ash::Device>,
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
            Texture::new(&create_info, memory_allocator, device)?
        };

        Ok(image)
    }

    pub fn transition_layout(
        &mut self,
        cmd: vk::CommandBuffer,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        transition_image_layout(
            cmd,
            self.image,
            self.aspect,
            old_layout,
            new_layout,
            self.device.as_ref(),
        );
    }

    pub fn copy_to_vkimage(
        &self,
        dst_image: vk::Image,
        dst_image_extent: vk::Extent2D,
        cmd: vk::CommandBuffer,
    ) {
        copy_vkimage_to_vkimage(
            cmd,
            self.image,
            dst_image,
            vk::Extent2D {
                width: self.extent.width,
                height: self.extent.height,
            },
            dst_image_extent,
            self.device.as_ref(),
        );
    }

    pub fn copy_to(&self, dst: &Texture, cmd: vk::CommandBuffer) {
        self.copy_to_vkimage(
            dst.image,
            vk::Extent2D {
                width: dst.extent.width,
                height: dst.extent.height,
            },
            cmd,
        );
    }

    fn upload(&mut self, data: &[u8], transfer: &TransferCommandEncoder) -> Result<()> {
        let mut staging_buffer = Buffer::new(
            data.len() as u64,
            256,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk_mem::MemoryUsage::AutoPreferHost,
            true,
            self.memory_allocator.clone(),
            self.device.clone(),
        )?;
        staging_buffer.write(data, 0)?;
        transfer.immediate_submit(|cmd: vk::CommandBuffer, device: &ash::Device| {
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
                // Create a pipeline barrier that blocks from
                // VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT to VK_PIPELINE_STAGE_TRANSFER_BIT
                // Read more: https://gpuopen.com/learn/vulkan-barriers-explained/
                device.cmd_pipeline_barrier(
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
                // Copy staging buffer into image
                device.cmd_copy_buffer_to_image(
                    cmd,
                    staging_buffer.buffer,
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

            // Barrier the image into the shader-readable layout
            unsafe {
                device.cmd_pipeline_barrier(
                    cmd,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
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
        unsafe {
            self.device.destroy_image_view(self.view, None);
            let allocation = self.allocation.as_mut().expect("Allocation does not exist");
            self.memory_allocator
                .lock()
                .expect("Failed to acquire lock for memory allocator")
                .destroy_image(self.image, allocation);
        }
    }
}

fn copy_vkimage_to_vkimage(
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

fn transition_image_layout(
    cmd: vk::CommandBuffer,
    image: vk::Image,
    image_aspect: vk::ImageAspectFlags,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
    device: &ash::Device,
) {
    if old_layout == new_layout {
        return;
    }

    let image_barrier = vk::ImageMemoryBarrier2 {
        src_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
        src_access_mask: vk::AccessFlags2::MEMORY_WRITE,
        dst_stage_mask: vk::PipelineStageFlags2::ALL_COMMANDS,
        dst_access_mask: vk::AccessFlags2::MEMORY_WRITE | vk::AccessFlags2::MEMORY_READ,
        old_layout,
        new_layout,
        subresource_range: vk::ImageSubresourceRange {
            aspect_mask: image_aspect,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        },
        image,
        ..Default::default()
    };

    let dep_info = vk::DependencyInfo {
        image_memory_barrier_count: 1,
        p_image_memory_barriers: &image_barrier,
        ..Default::default()
    };

    unsafe {
        device.cmd_pipeline_barrier2(cmd, &dep_info);
    }
}
