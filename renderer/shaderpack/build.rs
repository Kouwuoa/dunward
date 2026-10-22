extern crate shaderc;

use color_eyre::{Result, eyre::OptionExt, eyre::eyre};
use naga::{
    back::spv,
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
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
    let out_dir = env::var("OUT_DIR")?;
    let shaders_in_dir = Path::new(&cargo_manifest_dir).join("shaders");
    let shaders_out_dir = Path::new(&out_dir).join("shaders-built");

    let mut shader_entries = Vec::new();

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
            _ => {
                return Err(eyre!(
                    "Shader language not recognized for file: {:#?}",
                    path
                ));
            }
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
        let shader_filename = format!("{}.spv", shader_name);
        let output_filepath = shaders_out_dir.join(shader_filename.clone());
        fs::create_dir_all(output_filepath.parent().ok_or_eyre("No parent")?)?;
        fs::write(output_filepath, bytemuck::cast_slice(&spv_binary))?;

        let shader_id = to_shader_id(shader_name);
        shader_entries.push((shader_id, shader_filename))
    }

    // Sort to keep generated code deterministic across builds
    shader_entries.sort();
    generate_rust_code(&shaders_out_dir, &shader_entries)?;

    Ok(())
}

fn compile_glsl(filepath: &Path) -> Result<Vec<u32>> {
    let compiler = shaderc::Compiler::new()?;
    let options = shaderc::CompileOptions::new()?;

    let ext = filepath
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_eyre(format!("Shader file has no extension: {:#?}", filepath))?;

    let shader_kind = match ext {
        "vert" => ShaderKind::Vertex,
        "frag" => ShaderKind::Fragment,
        "comp" => ShaderKind::Compute,
        _ => {
            return Err(eyre!(
                "Shader kind not recognized for GLSL file: {:#?}",
                filepath
            ));
        }
    };

    let source = fs::read_to_string(&filepath)?;
    let filename = filepath
        .file_name()
        .ok_or_eyre(format!("No filename for filepath: {:#?}", filepath))?
        .to_str()
        .ok_or_eyre("Could not convert &OsStr to &str")?;
    let artifact =
        compiler.compile_into_spirv(&source, shader_kind, filename, "main", Some(&options))?;

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
    Ok(spv::write_vec(
        &module,
        &validation_info,
        &spv::Options::default(),
        None,
    )?)
}

// EX: "test-pattern.comp" -> "TestPatternComp"
fn to_shader_id(shader_name: &str) -> String {
    shader_name
        .split(|c: char| c == '-' || c == '_' || c == '.')
        .filter(|part| !part.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn generate_rust_code(shaders_out_dir: &Path, shader_entries: &[(String, String)]) -> Result<()> {
    let mut code = String::new();

    code.push_str(
        "/// Auto-generated ShaderId enum with each variant holding the name of the shader file\n\n",
    );
    code.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]\n");
    code.push_str("pub enum ShaderId {\n");
    for (shader_id, _) in shader_entries {
        code.push_str(&format!("    {},\n", shader_id));
    }
    code.push_str("}\n\n");

    code.push_str("impl ShaderId {\n");
    code.push_str("    pub const fn spv(self) -> &'static [u8] {\n");
    code.push_str("        match self {\n");
    for (shader_id, shader_filename) in shader_entries {
        code.push_str(&format!(
               "            Self::{} => include_bytes!(concat!(env!(\"OUT_DIR\"), \"/shaders-built/{}\")),\n",
               shader_id, shader_filename
           ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    let dest_path = shaders_out_dir.join("shaders_generated.rs");
    fs::write(dest_path, code)?;

    Ok(())
}
