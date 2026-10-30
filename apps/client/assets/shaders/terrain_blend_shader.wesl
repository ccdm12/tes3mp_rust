// For 2d replace `bevy_pbr::render::mesh_functions` with `bevy_sprite_render::mesh2d::functions`
// and `mesh_position_local_to_clip` with `mesh2d_position_local_to_clip`.

import bevy_pbr::{
    mesh_bindings::mesh,
    render::mesh_functions,
    render::forward_io::{Vertex, VertexOutput},
    render::view_transformations::position_world_to_clip
};
struct TerrainMaterialParams {
    num_textures_in_array: u32,
    diffuse_tiling_factor: f32,
    base_vtex_grid_dim: f32,
    padding: u32,
}

// --- Bind Groups ---
@group(constants::MATERIAL_BIND_GROUP) @binding(0)
var source_textures_array: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(1)
var tex_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(2)
var alpha_maps: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(3)
var alpha_map_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(4)
var<uniform> material: TerrainMaterialParams;

struct VertexInput {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(5) vertex_color: vec4<f32>,
}

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let mesh_world_from_local = mesh_functions::get_world_from_local(in.instance_index);
    var world_from_local = mesh_world_from_local;
    out.world_normal = mesh_functions::mesh_normal_local_to_world(in.normal, in.instance_index);

    out.world_position = mesh_functions::mesh_position_local_to_world(
            world_from_local,
            vec4<f32>(in.position, 1.0),
        );
    out.position = position_world_to_clip(out.world_position.xyz);
    out.uv = in.uv0;
    out.color = in.vertex_color;

    return out;
}

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let nudge_val = (1.0 / material.base_vtex_grid_dim) / 4.0;
    let nudge_offset_xy_for_shader = vec2<f32>(nudge_val, nudge_val);

    var alpha_uv = mesh.uv;
    let pre_scale_translate_xy = vec2<f32>(-0.5, -0.5);

    // Apply the full OpenMW BlendmapTexMat transformation sequence:
    alpha_uv = alpha_uv + nudge_offset_xy_for_shader;
    alpha_uv = alpha_uv + pre_scale_translate_xy;
    alpha_uv = alpha_uv * material.base_vtex_grid_dim / (material.base_vtex_grid_dim + 1.0);
    alpha_uv = alpha_uv - pre_scale_translate_xy;
    let tiled_uv = mesh.uv * material.diffuse_tiling_factor;
    var blended_diffuse_color_rgb = vec3<f32>(0.0, 0.0, 0.0);
    var total_weight_contribution = 0.0;
    let layers_to_sample = material.num_textures_in_array;

    if layers_to_sample == 0u {
        blended_diffuse_color_rgb = vec3<f32>(0.18);
    } else if material.num_textures_in_array == 1u {
        blended_diffuse_color_rgb = textureSample(source_textures_array, tex_sampler, tiled_uv, 0u)
                .rgb;
    } else {
        for(var i: u32 = 0u; i < layers_to_sample; i = i + 1u) {
            let diffuse_color_sample = textureSampleLevel(
                    source_textures_array,
                    tex_sampler,
                    tiled_uv,
                    i,
                    0.0, // Explicit mip level
                );
            let alpha_weight = textureSampleLevel(alpha_maps, alpha_map_sampler, alpha_uv, i, 0.0)
                    .r;
            blended_diffuse_color_rgb += diffuse_color_sample.rgb * alpha_weight;
            total_weight_contribution += alpha_weight;
        }

        if total_weight_contribution > 0.001 {
            blended_diffuse_color_rgb /= total_weight_contribution;
        } else {
            if layers_to_sample > 0u {
                blended_diffuse_color_rgb = textureSampleLevel(
                        source_textures_array,
                        tex_sampler,
                        tiled_uv,
                        0u,
                        0.0,
                    ).rgb;
            } else {
                blended_diffuse_color_rgb = vec3<f32>(0.18);
            }
        }
    }
    let albedo_linear_rgb = blended_diffuse_color_rgb;

    return vec4<f32>(albedo_linear_rgb, 1.0) * mesh.color;
}
