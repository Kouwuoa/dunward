#version 450
#extension GL_EXT_nonuniform_qualifier : require

struct PerFrameData {
    mat4 viewproj;
    float near;
    float far;
    float _padding[2];
};
struct PerMaterialData {
    uint texture_index;
    uint sampler_index;
};
struct PerObjectData {
    mat4 model;
};

layout(set = 0, binding = 0) uniform PerFrameBuffer {
    PerFrameData data;
} per_frame;
layout(set = 0, binding = 1) buffer PerMaterialBuffer {
    PerMaterialData data[];
} per_material;
layout(set = 0, binding = 2) buffer PerObjectBuffer {
    PerObjectData data[];
} per_object;
layout(set = 0, binding = 3) uniform sampler samplers[];
layout(set = 0, binding = 4) uniform texture2D textures[];

layout(push_constant) uniform PerDrawData {
    uint object_index;
    uint material_index;
} per_draw;

layout(location = 0) in vec3 in_position;
layout(location = 1) in vec2 in_texcoord;

layout(location = 0) out vec2 out_texcoord;

void main() {
    uint object_index = per_draw.object_index;
    uint material_index = per_draw.material_index;
    uint texture_index = per_material.data[material_index].texture_index;
    uint sampler_index = per_material.data[material_index].sampler_index;

    mat4 model = per_object.data[object_index].model;
    mat4 viewproj = per_frame.data.viewproj;

    gl_Position = viewproj * model * vec4(0.0, 0.0, 0.0, 1.0);
    out_texcoord = in_texcoord;
}