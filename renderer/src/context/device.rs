use super::{
    commands::CommandEncoderAllocator,
    commands::{CommandEncoderAllocatorExt, TransferCommandEncoder},
    instance::RenderInstance,
    queue::{Queue, QueueFamily},
};
use crate::context::commands::CommandEncoder;
use crate::resources::resource_type::RenderResourceType;
use crate::resources::texture::{ColorTexture, DepthTexture, StorageTexture};
use crate::resources::{
    megabuffer::{Megabuffer, MegabufferExt},
    texture::Texture,
};
use ash::vk;
use color_eyre::{Result, eyre::OptionExt};
use gpu_descriptor::DescriptorAllocator;
use std::ffi::{CStr, c_char};
use std::str::Utf8Error;
use std::sync::{Arc, Mutex};

/// Main way to submit rendering commands to the GPU.
pub(crate) struct RenderDevice {
    pub logical: Arc<ash::Device>,
    pub physical: vk::PhysicalDevice,

    // For now, require the graphics queue to support presentation
    graphics_queue: Arc<Queue>,
    compute_queue: Arc<Queue>,
    transfer_queue: Arc<Queue>,

    pub descriptor_allocator:
        Arc<Mutex<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>>,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    command_encoder_allocator: CommandEncoderAllocator,

    transfer: Arc<TransferCommandEncoder>,
}

impl RenderDevice {
    pub fn new(
        instance: &RenderInstance,
        surface: Option<(&vk::SurfaceKHR, &ash::khr::surface::Instance)>,
    ) -> Result<Self> {
        let (physical_device, graphics_queue_family, compute_queue_family, transfer_queue_family) =
            Self::select_physical_device(instance.inner(), surface)?;

        let (logical_device, graphics_queue, compute_queue, transfer_queue) =
            Self::create_logical_device(
                instance.inner(),
                &physical_device,
                graphics_queue_family,
                compute_queue_family,
                transfer_queue_family,
            )?;

        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                instance.inner(),
                &logical_device,
                physical_device,
            ))?
        };

        let logical_device = Arc::new(logical_device);
        let graphics_queue = Arc::new(graphics_queue);
        let compute_queue = Arc::new(compute_queue);
        let transfer_queue = Arc::new(transfer_queue);

        let command_encoder_allocator = CommandEncoderAllocator::new(logical_device.clone())?;
        let descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet> =
            DescriptorAllocator::new(
                RenderResourceType::max_update_after_bind_descriptors_in_all_pools(),
            );

        let transfer = TransferCommandEncoder::new(transfer_queue.clone(), logical_device.clone())?;

        let dev = Self {
            logical: logical_device,
            physical: physical_device,

            graphics_queue,
            compute_queue,
            transfer_queue,

            descriptor_allocator: Arc::new(Mutex::new(descriptor_allocator)),
            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            command_encoder_allocator,

            transfer: Arc::new(transfer),
        };

        Ok(dev)
    }

    pub fn immediate_submit<F>(&self, func: F) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer, &ash::Device) -> Result<()>,
    {
        self.transfer.immediate_submit(func)
    }

    pub fn create_megabuffer(
        &self,
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
    ) -> Result<Megabuffer> {
        Megabuffer::new(
            size,
            alignment,
            buf_usage,
            self.memory_allocator.clone(),
            self.logical.clone(),
            self.transfer.clone(),
        )
    }

    pub fn create_color_texture(
        &self,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
    ) -> Result<ColorTexture> {
        Texture::new_color_texture_from_bytes(
            width,
            height,
            data,
            use_dedicated_memory,
            self.memory_allocator.clone(),
            self.logical.clone(),
            &self.transfer.clone(),
        )
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> Result<DepthTexture> {
        Texture::new_depth_texture(
            width,
            height,
            self.memory_allocator.clone(),
            self.logical.clone(),
        )
    }

    pub fn create_storage_texture(
        &self,
        width: u32,
        height: u32,
        use_dedicated_memory: bool,
    ) -> Result<StorageTexture> {
        Texture::new_storage_texture(
            width,
            height,
            use_dedicated_memory,
            self.memory_allocator.clone(),
            self.logical.clone(),
        )
    }

    pub fn allocate_command_encoder(&mut self, queue: Arc<Queue>) -> Result<CommandEncoder> {
        self.command_encoder_allocator.allocate(queue)
    }

    pub fn get_present_queue(&self) -> Arc<Queue> {
        self.graphics_queue.clone()
    }

    pub fn get_graphics_queue(&self) -> Arc<Queue> {
        self.graphics_queue.clone()
    }

    pub fn get_compute_queue(&self) -> Arc<Queue> {
        self.compute_queue.clone()
    }

    pub fn get_transfer_queue(&self) -> Arc<Queue> {
        self.transfer_queue.clone()
    }

    fn select_physical_device(
        instance: &ash::Instance,
        surface: Option<(&vk::SurfaceKHR, &ash::khr::surface::Instance)>,
    ) -> Result<(vk::PhysicalDevice, QueueFamily, QueueFamily, QueueFamily)> {
        let req_device_exts = Self::get_required_device_extensions();
        let req_device_exts = req_device_exts
            .iter()
            .map(|ext| ext.to_str())
            .collect::<std::result::Result<Vec<&str>, Utf8Error>>()?;

        Ok(unsafe {
            instance
                .enumerate_physical_devices()?
                .into_iter()
                // Filter out devices that do not contain the required device extensions
                .filter(|device| {
                    let supported_extensions = instance
                        .enumerate_device_extension_properties(*device)
                        .map_or(Vec::new(), |exts| exts);

                    req_device_exts.iter().all(|req_ext| {
                        let req_ext_supported = supported_extensions
                            .iter()
                            .map(|sup_exts| sup_exts.extension_name.as_ptr())
                            .any(
                                |sup_ext| match (*req_ext, CStr::from_ptr(sup_ext).to_str()) {
                                    (req, Ok(sup)) => req == sup,
                                    _ => false,
                                },
                            );
                        if !req_ext_supported {
                            log::error!("Device extension not supported: {}", req_ext);
                        }
                        req_ext_supported
                    })
                })
                // Filter out devices that do not contain the required queues
                .filter_map(|device| {
                    let props = instance.get_physical_device_queue_family_properties(device);

                    let graphics_queue_family_index =
                        props.iter().enumerate().position(|(i, q)| {
                            let supports_graphics =
                                q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                            if let Some((surface, surface_loader)) = surface {
                                let supports_present = {
                                    surface_loader
                                        .get_physical_device_surface_support(
                                            device, i as u32, *surface,
                                        )
                                        .map_or(false, |b| b)
                                };
                                supports_graphics && supports_present
                            } else {
                                supports_graphics
                            }
                        });

                    let compute_queue_family_index = props.iter().enumerate().position(|(i, q)| {
                        let supports_compute = q.queue_flags.contains(vk::QueueFlags::COMPUTE);
                        let same_as_graphics = graphics_queue_family_index == Some(i);
                        supports_compute && !same_as_graphics
                    });

                    let transfer_queue_family_index =
                        props.iter().enumerate().position(|(i, q)| {
                            let supports_transfer =
                                q.queue_flags.contains(vk::QueueFlags::TRANSFER);
                            let same_as_graphics = graphics_queue_family_index == Some(i);
                            let same_as_compute = compute_queue_family_index == Some(i);
                            supports_transfer && !same_as_graphics && !same_as_compute
                        });

                    if let (
                        Some(graphics_queue_family_index),
                        Some(compute_queue_family_index),
                        Some(transfer_queue_family_index),
                    ) = (
                        graphics_queue_family_index,
                        compute_queue_family_index,
                        transfer_queue_family_index,
                    ) {
                        Some((
                            device,
                            graphics_queue_family_index as u32,
                            compute_queue_family_index as u32,
                            transfer_queue_family_index as u32,
                        ))
                    } else {
                        None
                    }
                })
                .min_by_key(|(device, _, _, _)| {
                    let props = instance.get_physical_device_properties(*device);
                    match props.device_type {
                        vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                        vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                        vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                        vk::PhysicalDeviceType::CPU => 3,
                        vk::PhysicalDeviceType::OTHER => 4,
                        _ => 5,
                    }
                })
                .map(
                    |(
                        device,
                        graphics_queue_family_index,
                        compute_queue_family_index,
                        transfer_queue_family_index,
                    )| {
                        let queue_family_props =
                            instance.get_physical_device_queue_family_properties(device);
                        let graphics_props = queue_family_props
                            .get(graphics_queue_family_index as usize)
                            .unwrap();
                        let compute_props = queue_family_props
                            .get(compute_queue_family_index as usize)
                            .unwrap();
                        let transfer_props = queue_family_props
                            .get(transfer_queue_family_index as usize)
                            .unwrap();
                        (
                            device,
                            QueueFamily::new(graphics_queue_family_index, *graphics_props, true),
                            QueueFamily::new(compute_queue_family_index, *compute_props, false),
                            QueueFamily::new(transfer_queue_family_index, *transfer_props, false),
                        )
                    },
                )
                .ok_or_eyre("No suitable physical device found")?
        })
    }

    fn create_logical_device(
        instance: &ash::Instance,
        physical_device: &vk::PhysicalDevice,
        graphics_queue_family: QueueFamily,
        compute_queue_family: QueueFamily,
        transfer_queue_family: QueueFamily,
    ) -> Result<(ash::Device, Queue, Queue, Queue)> {
        let queue_priorities = [1.0];
        let queue_create_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(graphics_queue_family.index)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(compute_queue_family.index)
                .queue_priorities(&queue_priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(transfer_queue_family.index)
                .queue_priorities(&queue_priorities),
        ];

        // Create device
        let device = {
            let enabled_extension_names = Self::get_required_device_extensions()
                .iter()
                .map(|ext| ext.as_ptr())
                .collect::<Vec<*const c_char>>();

            let mut features2 = vk::PhysicalDeviceFeatures2::default();
            unsafe {
                instance.get_physical_device_features2(*physical_device, &mut features2);
            }
            let mut features11 =
                vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);
            let mut features12 = vk::PhysicalDeviceVulkan12Features::default()
                .runtime_descriptor_array(true)
                .buffer_device_address(true)
                .descriptor_indexing(true)
                .descriptor_binding_partially_bound(true)
                .descriptor_binding_variable_descriptor_count(true)
                // Dynamic indexing
                .shader_input_attachment_array_dynamic_indexing(true)
                .shader_uniform_texel_buffer_array_dynamic_indexing(true)
                .shader_storage_texel_buffer_array_dynamic_indexing(true)
                // Non-uniform indexing
                .shader_uniform_buffer_array_non_uniform_indexing(true)
                .shader_sampled_image_array_non_uniform_indexing(true)
                .shader_storage_buffer_array_non_uniform_indexing(true)
                .shader_storage_image_array_non_uniform_indexing(true)
                .shader_input_attachment_array_non_uniform_indexing(true)
                .shader_uniform_texel_buffer_array_non_uniform_indexing(true)
                .shader_storage_texel_buffer_array_non_uniform_indexing(true)
                // Update after bind
                .descriptor_binding_uniform_buffer_update_after_bind(true)
                .descriptor_binding_sampled_image_update_after_bind(true)
                .descriptor_binding_storage_image_update_after_bind(true)
                .descriptor_binding_storage_buffer_update_after_bind(true)
                .descriptor_binding_uniform_texel_buffer_update_after_bind(true)
                .descriptor_binding_storage_texel_buffer_update_after_bind(true);
            let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
                .synchronization2(true)
                .dynamic_rendering(true);

            let mut shader_object_features =
                vk::PhysicalDeviceShaderObjectFeaturesEXT::default().shader_object(true);

            let device_create_info = vk::DeviceCreateInfo::default() //enabled_features.device_create_info()
                .push_next(&mut features2)
                .push_next(&mut features11)
                .push_next(&mut features12)
                .push_next(&mut features13)
                .push_next(&mut shader_object_features)
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&enabled_extension_names);

            unsafe { instance.create_device(*physical_device, &device_create_info, None)? }
        };

        let graphics_queue = unsafe {
            let queue = device.get_device_queue(graphics_queue_family.index, 0);
            Queue::new(graphics_queue_family, queue)
        };
        let compute_queue = unsafe {
            let queue = device.get_device_queue(compute_queue_family.index, 0);
            Queue::new(compute_queue_family, queue)
        };
        let transfer_queue = unsafe {
            let queue = device.get_device_queue(transfer_queue_family.index, 0);
            Queue::new(transfer_queue_family, queue)
        };

        Ok((device, graphics_queue, compute_queue, transfer_queue))
    }

    fn get_required_device_extensions() -> Vec<&'static CStr> {
        vec![
            ash::khr::swapchain::NAME,
            ash::khr::dynamic_rendering::NAME,
            ash::khr::buffer_device_address::NAME,
            ash::khr::synchronization2::NAME,
            ash::khr::maintenance3::NAME,
            ash::ext::descriptor_indexing::NAME,
            ash::ext::shader_object::NAME,
            #[cfg(target_os = "macos")]
            ash::khr::portability_subset::NAME,
        ]
    }
}
