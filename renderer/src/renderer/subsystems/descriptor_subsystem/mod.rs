pub(super) mod descriptor_allocator;
pub(super) mod descriptor_set_layout_builder;
pub(super) mod descriptor_writer;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::subsystems::descriptor_subsystem::descriptor_writer::DescriptorWriter;
use color_eyre::Result;
use descriptor_allocator::DescriptorAllocator;
use std::sync::{Arc, Mutex};

pub struct DescriptorSubsystem {
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
    descriptor_writer: DescriptorWriter<'static>,
}

impl DescriptorSubsystem {
    pub fn new(dvc: &DeviceContext) -> Result<Self> {
        let descriptor_allocator = Arc::new(Mutex::new(DescriptorAllocator::new(
            dvc.logical_device_handle(),
            1000,
        )?));
        let descriptor_writer = DescriptorWriter::default();
        Ok(Self {
            descriptor_allocator,
            descriptor_writer,
        })
    }
}
