use std::ffi::{c_char, c_void, CStr};
use std::str::Utf8Error;
use std::sync::{Arc, Mutex};
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::owo_colors::OwoColorize;
use color_eyre::Result;
use gpu_descriptor::{CreatePoolError, DescriptorAllocator, DescriptorDevice, DescriptorPoolCreateFlags, DescriptorTotalCount, DeviceAllocationError};
use crate::renderer::resources::image::Image;
use crate::renderer::resources::megabuffer::{Megabuffer, MegabufferExt};
use crate::renderer::contexts::device_ctx::command_encoder_allocator::{CommandEncoderAllocator, CommandEncoderAllocatorExt};
use crate::renderer::contexts::device_ctx::instance::RenderInstance;
use crate::renderer::contexts::device_ctx::queue::{Queue, QueueFamily};
use crate::renderer::contexts::device_ctx::transfer_ctx::TransferContext;

/// Main structure for the renderer
pub(super) struct RenderDevice {
    pub logical: Arc<ash::Device>,
    pub physical: vk::PhysicalDevice,

    // For now, require the graphics queue to support presentation
    pub graphics_queue: Arc<Queue>,
    pub compute_queue: Arc<Queue>,
    pub transfer_queue: Arc<Queue>,

    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    command_encoder_allocator: CommandEncoderAllocator,
    pub descriptor_allocator: Arc<Mutex<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>>,

    transfer_context: Arc<TransferContext>,
}

impl RenderDevice {
    pub fn new(
        instance: &RenderInstance,
        surface: Option<&(vk::SurfaceKHR, ash::khr::surface::Instance)>,
    ) -> Result<Self> {
        let (
            physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        ) = Self::select_physical_device(
            &instance.instance,
            surface,
        )?;

        let (
            logical_device,
            graphics_queue,
            compute_queue,
            transfer_queue,
        ) = Self::create_logical_device(
            &instance.instance,
            &physical_device,
            graphics_queue_family,
            compute_queue_family,
            transfer_queue_family,
        )?;

        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                &instance.instance,
                &logical_device,
                physical_device,
            ))?
        };

        let logical_device = Arc::new(logical_device);
        let graphics_queue = Arc::new(graphics_queue);
        let compute_queue = Arc::new(compute_queue);
        let transfer_queue = Arc::new(transfer_queue);

        let command_encoder_allocator = CommandEncoderAllocator::new(
            logical_device.clone(),
        )?;
        let descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>
            = DescriptorAllocator::new(1024);

        let transfer_context = TransferContext::new(
            transfer_queue.clone(),
            logical_device.clone(),
        )?;

        let dev = Self {
            logical: logical_device,
            physical: physical_device,

            graphics_queue,
            compute_queue,
            transfer_queue,

            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            command_encoder_allocator,
            descriptor_allocator: Arc::new(Mutex::new(descriptor_allocator)),

            transfer_context: Arc::new(transfer_context),
        };

        Ok(dev)
    }

    pub fn immediate_submit<F>(
        &self,
        func: F,
    ) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer, &ash::Device) -> Result<()>,
    {
        self.transfer_context.immediate_submit(func)
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
            self.transfer_context.clone(),
        )
    }

    pub fn create_color_image(
        &self,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
    ) -> Result<Image> {
        Image::new_color_image(
            width,
            height,
            data,
            use_dedicated_memory,
            self.memory_allocator.clone(),
            self.logical.clone(),
            &self.transfer_context.clone(),
        )
    }

    pub fn create_depth_image(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Image> {
        Image::new_depth_image(
            width,
            height,
            self.memory_allocator.clone(),
            self.logical.clone()
        )
    }

    fn select_physical_device(
        instance: &ash::Instance,
        surface: Option<&(vk::SurfaceKHR, ash::khr::surface::Instance)>,
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
                            .map(|sup_exts| {
                                sup_exts.extension_name.as_ptr()
                            })
                            .any(|sup_ext| {
                                match (*req_ext, CStr::from_ptr(sup_ext).to_str()) {
                                    (req, Ok(sup)) => req == sup,
                                    _ => false,
                                }
                            });
                        if !req_ext_supported {
                            log::error!("Device extension not supported: {}", req_ext);
                        }
                        req_ext_supported
                    })
                })
                // Filter out devices that do not contain the required queues
                .filter_map(|device| {
                    let props = instance
                        .get_physical_device_queue_family_properties(device);

                    let graphics_queue_family_index = props
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            let supports_graphics = q.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                            if let Some((surface, surface_loader)) = surface {
                                let supports_present = {
                                    surface_loader.get_physical_device_surface_support(
                                        device,
                                        i as u32,
                                        *surface,
                                    ).map_or(false, |b| b)
                                };
                                supports_graphics && supports_present
                            } else {
                                supports_graphics
                            }
                        });

                    let compute_queue_family_index = props
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            let supports_compute = q.queue_flags.contains(vk::QueueFlags::COMPUTE);
                            let same_as_graphics = graphics_queue_family_index == Some(i);
                            supports_compute && !same_as_graphics
                        });

                    let transfer_queue_family_index = props
                        .iter()
                        .enumerate()
                        .position(|(i, q)| {
                            let supports_transfer = q.queue_flags.contains(vk::QueueFlags::TRANSFER);
                            let same_as_graphics = graphics_queue_family_index == Some(i);
                            let same_as_compute = compute_queue_family_index == Some(i);
                            supports_transfer && !same_as_graphics && !same_as_compute
                        });

                    if let (
                        Some(graphics_queue_family_index),
                        Some(compute_queue_family_index),
                        Some(transfer_queue_family_index)
                    ) = (
                        graphics_queue_family_index,
                        compute_queue_family_index,
                        transfer_queue_family_index
                    ) {
                        Some((
                            device,
                            graphics_queue_family_index as u32,
                            compute_queue_family_index as u32,
                            transfer_queue_family_index as u32
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
                .map(|(
                          device,
                          graphics_queue_family_index,
                          compute_queue_family_index,
                          transfer_queue_family_index,
                      )| {
                    let queue_family_props = instance.get_physical_device_queue_family_properties(device);
                    let graphics_props = queue_family_props.get(graphics_queue_family_index as usize).unwrap();
                    let compute_props = queue_family_props.get(compute_queue_family_index as usize).unwrap();
                    let transfer_props = queue_family_props.get(transfer_queue_family_index as usize).unwrap();
                    (
                        device,
                        QueueFamily::new(graphics_queue_family_index, *graphics_props, true),
                        QueueFamily::new(compute_queue_family_index, *compute_props, false),
                        QueueFamily::new(transfer_queue_family_index, *transfer_props, false),
                    )
                })
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
            let mut features11 = vk::PhysicalDeviceVulkan11Features::default()
                .shader_draw_parameters(true);
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

            let device_create_info = vk::DeviceCreateInfo::default()//enabled_features.device_create_info()
                .push_next(&mut features2)
                .push_next(&mut features11)
                .push_next(&mut features12)
                .push_next(&mut features13)
                .queue_create_infos(&queue_create_infos)
                .enabled_extension_names(&enabled_extension_names);

            unsafe {
                instance.create_device(*physical_device, &device_create_info, None)?
            }
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

            #[cfg(target_os = "macos")]
            ash::khr::portability_subset::NAME,
        ]
    }
}

pub struct AshDescriptorDevice(pub Arc<ash::Device>);

impl From<Arc<ash::Device>> for AshDescriptorDevice {
    fn from(device: Arc<ash::Device>) -> Self {
        Self(device)
    }
}

impl DescriptorDevice<vk::DescriptorSetLayout, vk::DescriptorPool, vk::DescriptorSet>
for AshDescriptorDevice
{
    unsafe fn create_descriptor_pool(
        &self,
        descriptor_count: &DescriptorTotalCount,
        max_sets: u32,
        flags: gpu_descriptor::DescriptorPoolCreateFlags,
    ) -> Result<vk::DescriptorPool, CreatePoolError> {
        let mut array = [vk::DescriptorPoolSize::default(); 13];
        let mut len = 0;

        if descriptor_count.sampler != 0 {
            array[len].ty = vk::DescriptorType::SAMPLER;
            array[len].descriptor_count = descriptor_count.sampler;
            len += 1;
        }

        if descriptor_count.combined_image_sampler != 0 {
            array[len].ty = vk::DescriptorType::COMBINED_IMAGE_SAMPLER;
            array[len].descriptor_count = descriptor_count.combined_image_sampler;
            len += 1;
        }

        if descriptor_count.sampled_image != 0 {
            array[len].ty = vk::DescriptorType::SAMPLED_IMAGE;
            array[len].descriptor_count = descriptor_count.sampled_image;
            len += 1;
        }

        if descriptor_count.storage_image != 0 {
            array[len].ty = vk::DescriptorType::STORAGE_IMAGE;
            array[len].descriptor_count = descriptor_count.storage_image;
            len += 1;
        }

        if descriptor_count.uniform_texel_buffer != 0 {
            array[len].ty = vk::DescriptorType::UNIFORM_TEXEL_BUFFER;
            array[len].descriptor_count = descriptor_count.uniform_texel_buffer;
            len += 1;
        }

        if descriptor_count.storage_texel_buffer != 0 {
            array[len].ty = vk::DescriptorType::STORAGE_TEXEL_BUFFER;
            array[len].descriptor_count = descriptor_count.storage_texel_buffer;
            len += 1;
        }

        if descriptor_count.uniform_buffer != 0 {
            array[len].ty = vk::DescriptorType::UNIFORM_BUFFER;
            array[len].descriptor_count = descriptor_count.uniform_buffer;
            len += 1;
        }

        if descriptor_count.storage_buffer != 0 {
            array[len].ty = vk::DescriptorType::STORAGE_BUFFER;
            array[len].descriptor_count = descriptor_count.storage_buffer;
            len += 1;
        }

        if descriptor_count.uniform_buffer_dynamic != 0 {
            array[len].ty = vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC;
            array[len].descriptor_count = descriptor_count.uniform_buffer_dynamic;
            len += 1;
        }

        if descriptor_count.storage_buffer_dynamic != 0 {
            array[len].ty = vk::DescriptorType::STORAGE_BUFFER_DYNAMIC;
            array[len].descriptor_count = descriptor_count.storage_buffer_dynamic;
            len += 1;
        }

        if descriptor_count.input_attachment != 0 {
            array[len].ty = vk::DescriptorType::INPUT_ATTACHMENT;
            array[len].descriptor_count = descriptor_count.input_attachment;
            len += 1;
        }

        if descriptor_count.acceleration_structure != 0 {
            array[len].ty = vk::DescriptorType::ACCELERATION_STRUCTURE_KHR;
            array[len].descriptor_count = descriptor_count.acceleration_structure;
            len += 1;
        }

        if descriptor_count.inline_uniform_block_bytes != 0 {
            panic!("Inline uniform blocks are not supported");
        }

        if descriptor_count.inline_uniform_block_bindings != 0 {
            panic!("Inline uniform blocks are not supported");
        }

        let mut ash_flags = vk::DescriptorPoolCreateFlags::empty();

        if flags.contains(DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET) {
            ash_flags |= vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET;
        }

        if flags.contains(DescriptorPoolCreateFlags::UPDATE_AFTER_BIND) {
            ash_flags |= vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND;
        }

        let result = unsafe {
            self.0.create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::default()
                    .max_sets(max_sets)
                    .pool_sizes(&array[..len])
                    .flags(ash_flags),
                None,
            )
        };

        match result {
            Ok(pool) => Ok(pool),
            Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => Err(CreatePoolError::OutOfDeviceMemory),
            Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => Err(CreatePoolError::OutOfHostMemory),
            Err(vk::Result::ERROR_FRAGMENTATION) => Err(CreatePoolError::Fragmentation),
            Err(err) => panic!("Unexpected return code '{}'", err),
        }
    }

    unsafe fn destroy_descriptor_pool(&self, pool: vk::DescriptorPool) {
        unsafe {
            self.0.destroy_descriptor_pool(pool, None)
        }
    }

    unsafe fn alloc_descriptor_sets<'a>(
        &self,
        pool: &mut vk::DescriptorPool,
        layouts: impl ExactSizeIterator<Item = &'a vk::DescriptorSetLayout>,
        sets: &mut impl Extend<vk::DescriptorSet>,
    ) -> Result<(), DeviceAllocationError> {
        let set_layouts: smallvec::SmallVec<[_; 16]> = layouts.copied().collect();

        unsafe {
            match self.0.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::default()
                    .set_layouts(&set_layouts)
                    .descriptor_pool(*pool),
            ) {
                Ok(allocated) => {
                    sets.extend(allocated);
                    Ok(())
                }
                Err(vk::Result::ERROR_OUT_OF_HOST_MEMORY) => {
                    Err(DeviceAllocationError::OutOfHostMemory)
                }
                Err(vk::Result::ERROR_OUT_OF_DEVICE_MEMORY) => {
                    Err(DeviceAllocationError::OutOfDeviceMemory)
                }
                Err(vk::Result::ERROR_FRAGMENTED_POOL) => Err(DeviceAllocationError::OutOfPoolMemory),
                Err(vk::Result::ERROR_OUT_OF_POOL_MEMORY) => Err(DeviceAllocationError::FragmentedPool),
                Err(err) => panic!("Unexpected return code '{}'", err),
            }
        }
    }

    unsafe fn dealloc_descriptor_sets<'a>(
        &self,
        pool: &mut vk::DescriptorPool,
        sets: impl Iterator<Item = vk::DescriptorSet>,
    ) {
        let sets: smallvec::SmallVec<[_; 16]> = sets.collect();
        unsafe {
            match self.0.free_descriptor_sets(*pool, &sets) {
                Ok(()) => {}
                Err(err) => panic!("Unexpected return code '{}'", err),
            }
        }
    }
}