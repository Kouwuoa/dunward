//! Vulkan Swapchain lifecycle, image handles, and image view generation.
//!
//! Wraps [`ash::vk::SwapchainKHR`], querying surface capabilities, selecting
//! extent/formats, and constructing swapchain image views for window presentation.

use crate::core::DeviceContext;
use ash::vk;
use color_eyre::Result;
use winit::dpi::PhysicalSize;

pub(crate) type SwapchainImage = vk::Image;
#[allow(dead_code)]
pub(crate) type SwapchainImageIndex = u32;
pub(crate) type SwapchainImageExtent = vk::Extent2D;

pub(crate) struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    #[allow(dead_code)]
    pub swapchain_present_mode: vk::PresentModeKHR,
    pub swapchain_images: Vec<SwapchainImage>,
    #[allow(dead_code)]
    pub swapchain_image_count: u32,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_image_extent: SwapchainImageExtent,
    pub swapchain_image_format: vk::Format,
    #[allow(dead_code)]
    pub swapchain_image_color_space: vk::ColorSpaceKHR,
    #[allow(dead_code)]
    pub swapchain_image_usage: vk::ImageUsageFlags,
    #[allow(dead_code)]
    pub swapchain_image_sharing_mode: vk::SharingMode,
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
        }
    }
}

impl Swapchain {
    pub(super) fn new(
        size: &PhysicalSize<u32>,
        dvc_ctx: &DeviceContext,
        old_swapchain: Option<&Self>,
    ) -> Result<Self> {
        let surface_format = dvc_ctx.find_suitable_surface_format()?;
        let surface_present_mode = dvc_ctx.find_suitable_surface_present_mode();
        let surface_capabilities = dvc_ctx.get_physical_device_surface_capabilities()?;

        let image_extent = {
            if surface_capabilities.current_extent.width != u32::MAX {
                surface_capabilities.current_extent
            } else {
                vk::Extent2D {
                    width: size.width.clamp(
                        surface_capabilities.min_image_extent.width,
                        surface_capabilities.max_image_extent.width,
                    ),
                    height: size.height.clamp(
                        surface_capabilities.min_image_extent.height,
                        surface_capabilities.max_image_extent.height,
                    ),
                }
            }
        };

        let min_image_count = surface_capabilities.min_image_count + 1;
        let image_count = if surface_capabilities.max_image_count > 0 {
            min_image_count.min(surface_capabilities.max_image_count)
        } else {
            min_image_count
        };

        let queue_family_indices = [
            dvc_ctx.get_graphics_queue().family.index,
            dvc_ctx.get_present_queue().family.index,
        ];
        let image_sharing_mode = if queue_family_indices[0] != queue_family_indices[1] {
            vk::SharingMode::CONCURRENT
        } else {
            vk::SharingMode::EXCLUSIVE
        };

        let old_swapchain_handle = match old_swapchain {
            Some(old_swapchain) => old_swapchain.swapchain,
            None => vk::SwapchainKHR::null(),
        };

        let swapchain_loader = dvc_ctx.create_vk_swapchain_loader();
        let swapchain = unsafe {
            let mut info = vk::SwapchainCreateInfoKHR::default()
                .surface(dvc_ctx.raw_surface_handle())
                .min_image_count(image_count)
                .image_format(surface_format.format)
                .image_color_space(surface_format.color_space)
                .image_extent(image_extent)
                .image_array_layers(1)
                .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
                .image_sharing_mode(image_sharing_mode)
                .pre_transform(surface_capabilities.current_transform)
                .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                .present_mode(surface_present_mode)
                .clipped(true)
                .old_swapchain(old_swapchain_handle);

            if image_sharing_mode == vk::SharingMode::CONCURRENT {
                info = info.queue_family_indices(&queue_family_indices);
            }

            swapchain_loader.create_swapchain(&info, None)?
        };

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };
        let swapchain_image_views = swapchain_images
            .iter()
            .map(|&image| {
                let view_info = vk::ImageViewCreateInfo::default()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::IDENTITY,
                        g: vk::ComponentSwizzle::IDENTITY,
                        b: vk::ComponentSwizzle::IDENTITY,
                        a: vk::ComponentSwizzle::IDENTITY,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });
                dvc_ctx.create_vk_image_view(&view_info)
            })
            .collect::<Result<Vec<vk::ImageView>>>()?;

        Ok(Self {
            swapchain,
            swapchain_loader,
            swapchain_present_mode: surface_present_mode,
            swapchain_images,
            swapchain_image_count: image_count,
            swapchain_image_views,
            swapchain_image_extent: image_extent,
            swapchain_image_format: surface_format.format,
            swapchain_image_color_space: surface_format.color_space,
            swapchain_image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST,
            swapchain_image_sharing_mode: image_sharing_mode,
        })
    }
}
