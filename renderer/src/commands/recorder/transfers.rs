//! GPU Transfer commands, image blits, texture resolves, and clear operations.
//!
//! Implements transfer and image operations recorded onto a command buffer, including
//! [`blit_texture_to_texture`], [`resolve_texture`], [`clear_storage_texture`],
//! [`clear_color_texture`], and [`clear_depth_texture`].

use super::{CommandRecorder, Recording};
use crate::resources::texture::{
    ColorTexture, DepthTexture, StorageTexture, Texture, TextureAccess,
};
use ash::vk;
use color_eyre::Result;

impl CommandRecorder<Recording> {
    pub fn blit_texture_to_texture(&self, src: &mut Texture, dst: &mut Texture) -> Result<()> {
        src.blit_to(dst, self.command_buffer);
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
        texture: &mut StorageTexture,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        assert!(
            texture.layout == vk::ImageLayout::GENERAL
                || texture.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            "Cannot clear storage texture in {:?} layout. Texture must be in GENERAL or TRANSFER_DST_OPTIMAL layout",
            texture.layout
        );

        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                texture.layout,
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

        // Update tracked state so subsequent barriers know a clear (TRANSFER_WRITE) just occurred
        texture.access_state = TextureAccess {
            stage_mask: vk::PipelineStageFlags2::TRANSFER,
            access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        };

        Ok(())
    }

    pub fn clear_color_texture(
        &self,
        texture: &mut ColorTexture,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        assert!(
            texture.layout == vk::ImageLayout::GENERAL
                || texture.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            "Cannot clear color texture in {:?} layout. Texture must be in GENERAL or TRANSFER_DST_OPTIMAL layout",
            texture.layout
        );

        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                texture.layout,
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

        // Update tracked state so subsequent barriers know a clear (TRANSFER_WRITE) just occurred
        texture.access_state = TextureAccess {
            stage_mask: vk::PipelineStageFlags2::TRANSFER,
            access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        };

        Ok(())
    }

    pub fn clear_depth_texture(&self, texture: &mut DepthTexture) -> Result<()> {
        assert!(
            texture.layout == vk::ImageLayout::GENERAL
                || texture.layout == vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            "Cannot clear depth texture in {:?} layout. Texture must be in GENERAL or TRANSFER_DST_OPTIMAL layout",
            texture.layout
        );

        unsafe {
            self.device.cmd_clear_depth_stencil_image(
                self.command_buffer,
                texture.image,
                texture.layout,
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

        // Update tracked state so subsequent barriers know a clear (TRANSFER_WRITE) just occurred
        texture.access_state = TextureAccess {
            stage_mask: vk::PipelineStageFlags2::TRANSFER,
            access_mask: vk::AccessFlags2::TRANSFER_WRITE,
        };

        Ok(())
    }
}
