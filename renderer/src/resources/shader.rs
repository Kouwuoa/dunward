use ash::vk;
use color_eyre::Result;
use std::path::Path;
use std::sync::Arc;

const SHADERS_DIR: &str = "shaders-built";

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
        let vert_mod = create_shader_module(
            (&format!("{}/{}.vert.spv", SHADERS_DIR, shader_name)).as_ref(),
            &device,
        )?;
        let frag_mod = create_shader_module(
            (&format!("{}/{}.frag.spv", SHADERS_DIR, shader_name)).as_ref(),
            &device,
        )?;
        Ok(Self {
            vert_mod,
            frag_mod,
            device,
        })
    }
}

impl ComputeShader {
    pub fn new(shader_name: &str, device: Arc<ash::Device>) -> Result<Self> {
        let comp_mod = create_shader_module(
            (&format!("{}/{}.comp.spv", SHADERS_DIR, shader_name)).as_ref(),
            &device,
        )?;
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
    let code = std::fs::read(filepath)?;

    let shader_module_info =
        vk::ShaderModuleCreateInfo::default().code(bytemuck::cast_slice(&code));

    let shader_module = unsafe { device.create_shader_module(&shader_module_info, None)? };

    Ok(shader_module)
}
