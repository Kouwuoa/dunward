use std::time::Duration;
use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::device_context::semaphore::{BinarySemaphore, TimelineSemaphore};
use crate::renderer::contexts::frame_context::packet::FrameRenderPacket;
use crate::renderer::contexts::swapchain_context::SwapchainContext;
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::shader_data::PerDrawData;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;
use color_eyre::Result;

pub(super) struct FrameLightingStageOutput<'a> {
    pub target_tex: &'a StorageTexture,
}

pub(super) struct FrameLightingStage {
    recorder: Option<CommandRecorder<Idle>>,

    target_tex: StorageTexture,
    target_tex_needs_update: bool,

    material: Material,
    is_first_render: bool,
}

impl FrameLightingStage {
    pub fn new(
        dvc_ctx: &DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        let compute_queue = dvc_ctx.get_compute_queue();
        let recorder = Some(cmd_sys.command_recorder_allocator.allocate(compute_queue)?);

        let swc_size = swc_ctx.get_size();
        let target_tex = rsc_sys.resource_factory.create_storage_texture(
            swc_size.width,
            swc_size.height,
            true,
        )?;

        let material = rsc_sys
            .resource_store
            .compute_material_factory
            .create_material()?;

        Ok(Self {
            recorder,
            target_tex,
            target_tex_needs_update: true,
            material,
            is_first_render: true,
        })
    }

    pub fn render(
        &mut self,
        pkt: FrameRenderPacket,
        dvc: &DeviceContext,
        frame_completion_timeline: &TimelineSemaphore,
        timeline_wait_val: u64,
        timeline_signal_val: u64,
    ) -> Result<FrameLightingStageOutput<'_>> {
        // Record render commands
        let recorder = self.recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            let graphics_queue = dvc.get_graphics_queue();
            let compute_queue = dvc.get_compute_queue();

            // Transition render target texture to GENERAL layout
            recorder.insert_texture_memory_barrier(
                &self.target_tex,
                if self.is_first_render {
                    vk::ImageLayout::UNDEFINED
                } else {
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL
                },
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags2::NONE,
                vk::AccessFlags2::NONE,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                if self.is_first_render {
                    None
                } else {
                    Some(&graphics_queue)
                },
                if self.is_first_render {
                    None
                } else {
                    Some(&compute_queue)
                },
            );

            // Update the render target texture if it needs updating
            if self.target_tex_needs_update {
                let mut updater = recorder.create_resource_updater();
                updater.enqueue_update(
                    |builder| {
                        builder.set_render_target_texture(&self.target_tex);
                    },
                    &self.material,
                );
                updater.execute_updates();
                self.target_tex_needs_update = false;
            }

            // Clear render target texture
            recorder.clear_storage_texture(
                &self.target_tex,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [1.0f32, 0.0f32, 0.0f32, 1.0f32],
                },
            )?;

            // Insert memory barrier that waits until the storage texture has been fully cleared before continuing with read/write operations
            // This effectively performs a flush operation to ensure the render operations that follow do not operate on stale data
            recorder.insert_texture_memory_barrier(
                &self.target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                None,
                None,
            );

            // Compute render operations
            recorder.bind_material(&self.material);
            let per_draw_data = PerDrawData {
                time_sec: pkt.time_start.elapsed().as_secs_f32(),
                ..Default::default()
            };
            recorder.update_push_constants(&self.material, per_draw_data.as_bytes());
            let group_count_x = (self.target_tex.width() as f32 / 16.0).ceil() as u32;
            let group_count_y = (self.target_tex.height() as f32 / 16.0).ceil() as u32;
            recorder.dispatch(group_count_x, group_count_y, 1);

            // Transition render target texture to transfer source layout to prepare for copying onto swapchain image
            // Also transfer the queue of the texture from compute to graphics to match the queue of the swapchain image
            recorder.insert_texture_memory_barrier(
                &self.target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_READ,
                Some(&compute_queue),
                Some(&graphics_queue),
            );

            Ok(())
        })?;

        self.recorder = Some(
            dvc.submit(
                recorder,
                &[frame_completion_timeline
                    .to_wait_semaphore(vk::PipelineStageFlags::COMPUTE_SHADER, timeline_wait_val)],
                &[frame_completion_timeline.to_signal_semaphore(timeline_signal_val)],
                None,
            )?,
        );

        self.is_first_render = false;

        Ok(FrameLightingStageOutput {
            target_tex: &self.target_tex,
        })
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, rsc_sys: &ResourceSubsystem) {
        self.target_tex = rsc_sys
            .resource_factory
            .create_storage_texture(size.width, size.height, true)
            .unwrap();
        self.target_tex_needs_update = true;
    }
}
