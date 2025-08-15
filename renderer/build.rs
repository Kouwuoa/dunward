extern crate shaderc;

use color_eyre::{
    Result,
    eyre::OptionExt,
    eyre::eyre
};
use naga::{
    back::spv, front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator}
};
use shaderc::ShaderKind;
use std::{env, fs, path::Path};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=shaders/*");

    compile_shaders()?;

    Ok(())
}

enum ShaderLanguage {
    Glsl,
    Wgsl,
}

fn compile_shaders() -> Result<()> {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
    let shaders_in_dir = Path::new(&cargo_manifest_dir).join("shaders");
    let shaders_out_dir = Path::new(&cargo_manifest_dir).join("shaders-built");

    for entry in fs::read_dir(shaders_in_dir)? {
        let entry = entry?;
        let path = entry.path();

        let ext = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_eyre(format!("Shader file has no extension: {:#?}", path))?;

        let shader_lang = match ext {
            "vert" => ShaderLanguage::Glsl,
            "frag" => ShaderLanguage::Glsl,
            "comp" => ShaderLanguage::Glsl,
            "wgsl" => ShaderLanguage::Wgsl,
            _ => return Err(eyre!("Shader language not recognized for file: {:#?}", path)),
        };

        let spv_binary = match shader_lang {
            ShaderLanguage::Glsl => compile_glsl(&path)?,
            ShaderLanguage::Wgsl => compile_wgsl(&path)?,
        };

        // Write the SPIR-V binary to a file
        let shader_name = path
            .file_name()
            .ok_or_eyre("Shader file has no name")?
            .to_str()
            .ok_or_eyre("Shader file name is not valid UTF-8")?;
        let output_filepath = shaders_out_dir
            .join(format!("{}.spv", shader_name));
        fs::create_dir_all(output_filepath.parent().ok_or_eyre("No parent")?)?;
        fs::write(output_filepath, bytemuck::cast_slice(&spv_binary))?;
    }

    Ok(())
}

fn compile_glsl(filepath: &Path) -> Result<Vec<u32>> {
    let compiler = shaderc::Compiler::new()
        .ok_or_eyre("Failed to create shaderc compiler")?;
    let options = shaderc::CompileOptions::new()
        .ok_or_eyre("Failed to create shaderc compile options")?;

    let ext = filepath
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_eyre(format!("Shader file has no extension: {:#?}", filepath))?;

    let shader_kind = match ext {
        "vert" => ShaderKind::Vertex,
        "frag" => ShaderKind::Fragment,
        "comp" => ShaderKind::Compute,
        _ => {
            return Err(eyre!("Shader kind not recognized for GLSL file: {:#?}", filepath));
        }
    };

    let source = fs::read_to_string(&filepath)?;
    let filename = filepath
        .file_name()
        .ok_or_eyre(format!("No filename for filepath: {:#?}", filepath))?
        .to_str()
        .ok_or_eyre("Could not convert &OsStr to &str")?;
    let artifact = compiler.compile_into_spirv(
        &source,
        shader_kind,
        filename,
        "main",
        Some(&options),
    )?;

    Ok(artifact.as_binary().to_vec())
}

fn compile_wgsl(filepath: &Path) -> Result<Vec<u32>> {
    // Read the WGSL file and parse into IR
    let source = fs::read_to_string(&filepath)?;
    let module = wgsl::parse_str(&source)?;

    // Validate the IR
    let mut validator = Validator::new(ValidationFlags::all(), Capabilities::all());
    let validation_info = validator.validate(&module)?;
    log::info!("{:#?}", validation_info);

    // Generate the SPIR-V binary
    Ok(spv::write_vec(&module, &validation_info, &spv::Options::default(), None)?)
}