use crate::viewport::PresentImage;

/// This struct is used to pass all necessary data for rendering a single frame.
/// It contains a payload with data about the objects to render
/// as well as metadata about the frame itself.
/// This struct should be created anew for each frame render call to ensure that the most up-to-date
/// context and storage are used, and it is lightweight enough to be cheap to create and pass around.
/// It is not meant to be stored or used outside the scope of a single frame render call
pub(crate) struct FrameRenderPacket<'a> {
    pub(super) payload: FrameRenderPayload<'a>,
    pub(super) metadata: FrameRenderMetadata,
}

/// This struct is used to pass necessary data to the render function of each frame.
/// This struct is not meant to be stored or used outside the scope of a single frame render call.
/// It should be created anew for each frame render to ensure that the most up-to-date context and storage are used.
/// It is also a lightweight struct that holds references, so it is cheap to create and pass around.
pub(super) struct FrameRenderPayload<'a> {
    pub(super) cam: &'a crate::Camera,
}

/// This struct is used to pass metadata about the frame being rendered.
/// This struct is not meant to be stored or used outside the scope of a single frame render call.
/// It should be created anew for each frame render to ensure that the most up-to-date context
/// and storage are used.
pub(super) struct FrameRenderMetadata {
    pub(super) frame_index: usize,
    pub(super) target_size: winit::dpi::PhysicalSize<u32>,
    pub(super) resize_requested: bool,
}

pub(crate) struct FramePresentPacket {
    pub(super) image: PresentImage,
}
