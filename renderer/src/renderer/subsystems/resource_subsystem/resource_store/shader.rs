use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::OptionExt;
use rust_embed::RustEmbed;
use std::path::Path;
use std::sync::Arc;

#[derive(RustEmbed)]
#[folder = "shaders-built/"]
struct ShadersEmbed;

pub struct GraphicsShader {
    pub vert_mod: vk::ShaderModule,
    pub frag_mod: vk::ShaderModule,
    device: Arc<ash::Device>,
}

pub struct ComputeShader {
    pub comp_mod: vk::ShaderModule,
    device: Arc<ash::Device>,
}

impl GraphicsShader {
    pub fn new(shader_name: &str, device: Arc<ash::Device>) -> Result<Self> {
        let vert_mod =
            create_shader_module((&format!("{}.vert.spv", shader_name)).as_ref(), &device)?;
        let frag_mod =
            create_shader_module((&format!("{}.frag.spv", shader_name)).as_ref(), &device)?;
        Ok(Self {
            vert_mod,
            frag_mod,
            device,
        })
    }
}

impl ComputeShader {
    pub fn new(shader_name: &str, device: Arc<ash::Device>) -> Result<Self> {
        let comp_mod =
            create_shader_module((&format!("{}.comp.spv", shader_name)).as_ref(), &device)?;
        Ok(Self { comp_mod, device })
    }
}

impl Drop for GraphicsShader {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.vert_mod, None);
            self.device.destroy_shader_module(self.frag_mod, None);
        }
    }
}

impl Drop for ComputeShader {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.comp_mod, None);
        }
    }
}

fn create_shader_module(filepath: &Path, device: &ash::Device) -> Result<vk::ShaderModule> {
    log::info!("Creating shader module from file: {:?}", filepath);

    let filepath = filepath.to_str().ok_or_eyre("Invalid shader file path")?;
    let embedded_file =
        ShadersEmbed::get(filepath).ok_or_eyre("Shader not found in embedded resources")?;
    let bytes = embedded_file.data;

    assert_eq!(bytes.len() % 4, 0, "Shader bytecode must be a multiple of 4 bytes");

    let shader_module_info =
        vk::ShaderModuleCreateInfo::default().code(bytemuck::cast_slice(&bytes));

    let shader_module = unsafe { device.create_shader_module(&shader_module_info, None)? };

    Ok(shader_module)
}
