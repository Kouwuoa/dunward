use crate::context::RenderContext;
use crate::storage::RenderStorage;
use std::sync::RwLockReadGuard;

/// This struct is used to pass all necessary data for rendering a single frame.
/// It contains a payload with data about the objects to render
/// as well as metadata about the frame itself.
/// This struct should be created anew for each frame render call to ensure that the most up-to-date
/// context and storage are used, and it is lightweight enough to be cheap to create and pass around.
/// It is not meant to be stored or used outside the scope of a single frame render call
pub(crate) struct FrameRenderPacket<'a> {
    pub payload: FrameRenderPayload<'a>,
    pub metadata: FrameRenderMetadata,
}

/// This struct is used to pass necessary data to the render function of each frame.
/// This struct is not meant to be stored or used outside the scope of a single frame render call.
/// It should be created anew for each frame render to ensure that the most up-to-date context and storage are used.
/// It is also a lightweight struct that holds references, so it is cheap to create and pass around.
pub(crate) struct FrameRenderPayload<'a> {
    pub cam: &'a crate::Camera,
}

/// This struct is used to pass metadata about the frame being rendered.
/// This struct is not meant to be stored or used outside the scope of a single frame render call.
/// It should be created anew for each frame render to ensure that the most up-to-date context
/// and storage are used.
pub(crate) struct FrameRenderMetadata {
    pub frame_index: usize,
    pub target_size: winit::dpi::PhysicalSize<u32>,
    pub resize_requested: bool,
}

pub(crate) struct FramePresentPacket {
    pub(super) swapchain_image_index: u32,
}
