pub(crate) mod commands;
pub(crate) mod desc_set_layout_builder;
pub(crate) mod device;
pub(crate) mod instance;
pub(crate) mod queue;

use crate::viewport::RenderViewport;
use ash::vk;
use color_eyre::Result;
use std::time::Duration;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct RenderContext {
    ins: instance::RenderInstance,
    dev: device::RenderDevice,
}

impl RenderContext {
    pub fn new(win: &winit::window::Window) -> Result<(Self, RenderViewport)> {
        log::info!("Creating RenderContext");

        let ins = instance::RenderInstance::new(Some(win))?;
        let sfc = ins.create_surface(win)?;
        let dev = ins.create_device(&sfc)?;
        let vpt = ins.create_viewport(sfc, win, &dev)?;

        Ok((Self { ins, dev }, vpt))
    }

    pub fn wait_and_reset_fence(&self, fence: vk::Fence, timeout: Duration) -> Result<()> {
        unsafe {
            let fences = [fence];
            self.dev
                .logical
                .wait_for_fences(&fences, true, timeout.as_nanos() as u64)?;
            self.dev.logical.reset_fences(&fences)?;
        }
        Ok(())
    }
}
