#version 450
#extension GL_EXT_buffer_reference : require
#extension GL_EXT_scalar_block_layout : require
#extension GL_EXT_nonuniform_qualifier : require


// These data structures must match exactly with Rust #[repr(C)]
struct PerFrameData {
    mat4 viewproj;
    float near;
    float far;
    float time_sec;
    float _padding;
};
struct PerMaterialData {
    uint texture_index;
    uint sampler_index;
};
struct PerObjectData {
    mat4 model;
};

// Declare buffer references (these are 64-bit pointers)
layout(buffer_reference, scalar) readonly buffer FrameBufferRef {
    PerFrameData frame;
};
layout(buffer_reference, scalar) readonly buffer MaterialBufferRef {
    PerMaterialData materials[];
};
layout(buffer_reference, scalar) readonly buffer ObjectBufferRef {
    PerObjectData objects[];
};

layout(push_constant) uniform PerDrawData {
    uint object_index;
    uint material_index;
} per_draw;

layout(set = 1, binding = 0) uniform sampler global_samplers[];
layout(set = 1, binding = 1) uniform texture2D global_textures[];

layout(location = 0) in vec2 in_texcoord;
layout(location = 0) out vec4 out_color;

void main() {
    uint object_index = per_draw.object_index;
    uint material_index = per_draw.material_index;
    uint texture_index = per_material.data[material_index].texture_index;
    uint sampler_index = per_material.data[material_index].sampler_index;

    out_color = texture(
        sampler2D(
        textures[nonuniformEXT(texture_index)],
        samplers[nonuniformEXT(sampler_index)]
        ),
        in_texcoord
    );
}
