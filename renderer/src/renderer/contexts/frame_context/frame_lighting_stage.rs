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

    finished_fence: vk::Fence,

    material: Material,
}

impl FrameLightingStage {
    const TIMELINE_SEM_SIGNAL_VALUE: u64 = 1;

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

        let finished_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        let material = rsc_sys
            .resource_store
            .compute_material_factory
            .create_material()?;

        Ok(Self {
            recorder,
            target_tex,
            target_tex_needs_update: true,
            finished_semaphore,
            finished_fence,
            material,
        })
    }

    pub fn render(
        &mut self,
        pkt: FrameRenderPacket,
        dvc: &DeviceContext,
        swc: &SwapchainContext,
        rsc: &ResourceSubsystem,
        frame_completion_timeline: &TimelineSemaphore,
        geometry_complete_timeline_value: u64,
        lighting_complete_timeline_value: u64,
    ) -> Result<FrameLightingStageOutput<'_>> {
        // Record render commands
        let recorder = self.recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            // Transition render target texture to GENERAL layout
            recorder.transition_texture_layout(
                &mut self.target_tex,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            )?;

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
            recorder.insert_texture_memory_barrier(
                &self.target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
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
            recorder.transition_texture_layout(
                &mut self.target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            )?;

            Ok(())
        })?;

        self.recorder = Some(dvc.submit(
            recorder,
            Some(frame_completion_timeline.to_wait_semaphore(
                vk::PipelineStageFlags::COMPUTE_SHADER,
                geometry_complete_timeline_value,
            )),
            Some(frame_completion_timeline.to_signal_semaphore(lighting_complete_timeline_value)),
            self.finished_fence,
        )?);

        Ok(FrameLightingStageOutput {
            target_tex: &self.target_tex,
        })
    }
}
