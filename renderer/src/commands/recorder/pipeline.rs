//! Pipeline state binding, push constant updates, compute dispatch, and resource updaters.
//!
//! Implements execution commands recorded onto a command buffer, including
//! [`bind_material`], [`update_push_constants`], [`dispatch`], and [`create_resource_updater`].

use super::{CommandRecorder, Recording};
use crate::material::Material;
use crate::resources::updater::ResourceUpdater;

impl CommandRecorder<Recording> {
    /// Binds material pipeline and descriptor sets to the command buffer.
    pub fn bind_material(&self, material: &Material) {
        material.bind(self.command_buffer);
    }

    /// Updates push constant ranges on the bound pipeline layout.
    pub fn update_push_constants(&self, material: &Material, data: &[u8]) {
        material.update_push_constants(self.command_buffer, data);
    }

    /// Dispatches compute shader workgroups.
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

    /// Creates a [`ResourceUpdater`] for batching descriptor set updates during recording.
    pub fn create_resource_updater(&self) -> ResourceUpdater<'_> {
        ResourceUpdater::new(&self.device, &self.command_buffer)
    }
}
