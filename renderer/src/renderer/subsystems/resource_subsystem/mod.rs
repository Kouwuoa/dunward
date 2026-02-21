use crate::renderer::contexts::device_context::DeviceContext;
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};

pub(crate) mod resource_binder;
pub(crate) mod resource_store;

pub(crate) struct ResourceSubsystem {
    pub memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
}

impl ResourceSubsystem {
    pub fn new(dvc: &DeviceContext) -> Result<Self> {
        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                &dvc.instance_handle(),
                &dvc.logical_device_handle(),
                dvc.physical_device_handle(),
            ))?
        };

        Ok(Self {
            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
        })
    }
}
