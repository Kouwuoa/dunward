//! Per-frame rendering and presentation packets.
//!
//! Exposes [`FrameRenderPacket`] containing scene, frame index, and timing data,
//! and [`FramePresentPacket`] encapsulating the acquired swapchain target.

use std::time::Instant;

use crate::display::PresentTextureBundle;

/// This struct is used to pass all necessary data for rendering a single frame.
/// It contains a payload with data about the objects to render
/// as well as metadata about the frame itself.
/// This struct should be created anew for each frame render call to ensure that the most up-to-date
/// context and storage are used, and it is lightweight enough to be cheap to create and pass around.
/// It is not meant to be stored or used outside the scope of a single frame render call
pub(crate) struct FrameRenderPacket<'a> {
    pub(crate) camera: &'a crate::Camera,
    pub(crate) target_size: winit::dpi::PhysicalSize<u32>,
    pub(crate) frame_index: usize,
    pub(crate) frame_number: u64,
    pub(crate) time_start: Instant,
}

pub(crate) struct FramePresentPacket {
    pub(crate) texture: PresentTextureBundle,
}
