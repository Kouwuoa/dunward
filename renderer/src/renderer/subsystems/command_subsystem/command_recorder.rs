use super::super::queue::Queue;
use super::command_recorder_allocator::{CommandRecorderAllocator, CommandRecorderAllocatorExt};
use crate::renderer::resource_store::material::Material;
use crate::renderer::resource_store::texture::{
    ColorTexture, DepthTexture, StorageTexture, Texture,
};
use ash::vk;
use color_eyre::Result;
use std::marker::PhantomData;
use std::sync::Arc;

/// Ready to record
pub(crate) struct Idle;
/// Currently recording
pub(crate) struct Recording;
/// Recording finished; can submit
pub(crate) struct Executable;

pub(crate) struct CommandRecorder<State> {
    pub(in crate::renderer::contexts::device_context) command_buffer: vk::CommandBuffer,
    pub(in crate::renderer::contexts::device_context) queue: Arc<Queue>,

    device: Arc<ash::Device>,
    /// Note that this is only an `Option` to allow for the allocator to be dropped.
    allocator: Option<CommandRecorderAllocator>,

    _state: PhantomData<State>,
}

impl CommandRecorder<Idle> {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        queue: Arc<Queue>,
        device: Arc<ash::Device>,
        allocator: CommandRecorderAllocator,
    ) -> Self {
        Self {
            command_buffer,
            queue,
            device,
            allocator: Some(allocator),
            _state: PhantomData,
        }
    }

    pub fn new_from_old(old: CommandRecorder<Executable>) -> Self {
        Self {
            command_buffer: old.command_buffer,
            queue: old.queue,
            device: old.device,
            allocator: old.allocator,
            _state: PhantomData,
        }
    }

    pub fn record<F>(self, f: F) -> Result<CommandRecorder<Executable>>
    where
        F: FnOnce(&CommandRecorder<Recording>) -> Result<()>,
    {
        let recorder = self.begin_recording()?;

        let result = f(&recorder);

        let recorder = recorder.end_recording();

        // Propagate user error first
        result?;

        recorder
    }

    fn begin_recording(self) -> Result<CommandRecorder<Recording>> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer, &begin_info)?;
        }

        Ok(CommandRecorder::<Recording> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}

impl CommandRecorder<Recording> {
    pub fn transition_texture_layout(
        &self,
        texture: &mut Texture,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> Result<()> {
        texture.transition_layout(self.command_buffer, old_layout, new_layout);

        Ok(())
    }

    pub fn copy_texture_to_texture(&self, src: &Texture, dst: &Texture) -> Result<()> {
        src.copy_to(dst, self.command_buffer);

        Ok(())
    }

    pub fn resolve_texture(
        &self,
        src: &Texture,
        src_layout: vk::ImageLayout,
        dst: &Texture,
        dst_layout: vk::ImageLayout,
        region: vk::ImageResolve,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_resolve_image(
                self.command_buffer,
                src.image,
                src_layout,
                dst.image,
                dst_layout,
                &[region],
            );
        }

        Ok(())
    }

    pub fn clear_storage_texture(
        &self,
        texture: &StorageTexture,
        current_layout: vk::ImageLayout,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                current_layout,
                color,
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    pub fn clear_color_texture(
        &self,
        texture: &ColorTexture,
        current_layout: vk::ImageLayout,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                current_layout,
                color,
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    pub fn clear_depth_texture(
        &self,
        texture: &DepthTexture,
        layout: vk::ImageLayout,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_depth_stencil_image(
                self.command_buffer,
                texture.image,
                layout,
                &vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    pub fn bind_material(&self, material: &Material) {
        material.bind_pipeline(self.command_buffer);
        material.bind_descriptor_sets(self.command_buffer);
    }

    pub fn update_push_constants(&self, material: &Material, data: &[u8]) {
        material.update_push_constants(self.command_buffer, data);
    }

    pub fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.device.cmd_dispatch(
                self.command_buffer,
                group_count_x,
                group_count_y,
                group_count_z,
            )
        }
    }

    fn end_recording(self) -> Result<CommandRecorder<Executable>> {
        unsafe { self.device.end_command_buffer(self.command_buffer)? }
        Ok(CommandRecorder::<Executable> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}
