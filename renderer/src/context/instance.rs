use super::device::RenderDevice;
use super::target::RenderTarget;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::{CStr, FromBytesUntilNulError, c_char, c_void};
use std::sync::Arc;
use winit::window::Window;

/// Initializes Vulkan and keeps the Vulkan instance alive
pub(super) struct RenderInstance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
    pub debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    pub debug_utils_loader: ash::ext::debug_utils::Instance,
}

impl RenderInstance {
    const ENABLE_VALIDATION_LAYERS: bool = cfg!(debug_assertions);
    const REQUIRED_VALIDATION_LAYERS: &'static [&'static CStr] = unsafe {
        &[CStr::from_bytes_with_nul_unchecked(
            b"VK_LAYER_KHRONOS_validation\0",
        )]
    };

    pub fn new(window: Option<&Window>) -> Result<Self> {
        let entry = unsafe { ash::Entry::load() }?;

        let instance = Self::create_instance(&entry, window)?;

        let (debug_utils_messenger, debug_utils_loader) =
            Self::create_debug_utils_messenger(&entry, &instance)?;

        Ok(Self {
            instance,
            entry,
            debug_utils_messenger,
            debug_utils_loader,
        })
    }

    pub fn create_device(
        &self,
        surface: Option<&(vk::SurfaceKHR, ash::khr::surface::Instance)>,
    ) -> Result<RenderDevice> {
        RenderDevice::new(self, surface)
    }

    pub fn create_surface(
        &self,
        window: &Window,
    ) -> Result<(vk::SurfaceKHR, ash::khr::surface::Instance)> {
        let surface = unsafe {
            ash_window::create_surface(
                &self.entry,
                &self.instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };
        let surface_loader = ash::khr::surface::Instance::new(&self.entry, &self.instance);
        Ok((surface, surface_loader))
    }

    pub fn create_target(
        &self,
        window: Arc<Window>,
        surface: (vk::SurfaceKHR, ash::khr::surface::Instance),
        dev: &RenderDevice,
    ) -> Result<RenderTarget> {
        RenderTarget::new(window, surface, self, dev)
    }

    fn create_instance(entry: &ash::Entry, window: Option<&Window>) -> Result<ash::Instance> {
        if Self::ENABLE_VALIDATION_LAYERS {
            Self::check_validation_layers_supported(entry)?;
        }

        let application_info = vk::ApplicationInfo::default().api_version(vk::API_VERSION_1_3);
        let enabled_layer_names = if Self::ENABLE_VALIDATION_LAYERS {
            Self::REQUIRED_VALIDATION_LAYERS
                .iter()
                .map(|layer| layer.as_ptr())
                .collect::<Vec<*const c_char>>()
        } else {
            Vec::new()
        };
        let enabled_extension_names = Self::get_required_instance_extensions(window)?
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<*const c_char>>();
        let mut debug_info = debug_utils_messenger_create_info();
        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&enabled_layer_names)
            .enabled_extension_names(&enabled_extension_names)
            .push_next(&mut debug_info);

        #[cfg(target_os = "macos")]
        let instance_info = instance_info.flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);

        Ok(unsafe { entry.create_instance(&instance_info, None)? })
    }

    fn create_debug_utils_messenger(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> Result<(vk::DebugUtilsMessengerEXT, ash::ext::debug_utils::Instance)> {
        let debug_utils_loader = ash::ext::debug_utils::Instance::new(entry, instance);
        let debug_utils_info = debug_utils_messenger_create_info();
        let debug_utils_messenger =
            unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_utils_info, None)? };
        Ok((debug_utils_messenger, debug_utils_loader))
    }

    fn get_required_instance_extensions(
        window: Option<&Window>,
    ) -> Result<Vec<&'static CStr>> {
        let mut exts = if let Some(window) = window {
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?
                .iter()
                .map(|ext| unsafe { CStr::from_ptr(*ext) })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        if Self::ENABLE_VALIDATION_LAYERS {
            exts.push(ash::ext::debug_utils::NAME);
        }

        #[cfg(target_os = "macos")]
        {
            exts.push(ash::khr::portability_enumeration::NAME);
            exts.push(ash::khr::get_physical_device_properties2::NAME);
        }

        Ok(exts)
    }

    fn check_validation_layers_supported(entry: &ash::Entry) -> Result<()> {
        let layer_props = unsafe { entry.enumerate_instance_layer_properties()? };
        let supported_layers = layer_props
            .iter()
            .map(|props| props.layer_name_as_c_str())
            .collect::<std::result::Result<Vec<&CStr>, FromBytesUntilNulError>>()?;

        for layer in Self::REQUIRED_VALIDATION_LAYERS {
            if !supported_layers.contains(layer) {
                return Err(eyre!("Validation layer {:?} not supported", layer));
            }
        }

        Ok(())
    }
}
fn debug_utils_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
    let message_severity = vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
        | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR;
    let message_type = vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
        | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
        | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE;
    vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(message_severity)
        .message_type(message_type)
        .pfn_user_callback(Some(debug_callback))
}

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let msg_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
        _ => "[Unknown]",
    };
    let msg = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::trace!("[Verbose]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[Warning]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[Error]{} {:?}", msg_type, msg);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[Info]{} {:?}", msg_type, msg);
        }
        _ => {
            log::warn!("[Unknown]{} {:?}", msg_type, msg);
        }
    }

    vk::FALSE
}
