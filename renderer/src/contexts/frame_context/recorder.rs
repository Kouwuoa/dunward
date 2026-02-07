pub struct FrameRecorder<'a> {
    frm_ctx: &'a mut FrameContext,
    swc_ctx: &'a SwapchainContext,
    rsc_sto: &'a ResourceStore,
}