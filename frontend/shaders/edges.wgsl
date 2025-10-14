struct VsOut { @builtin(position) pos: vec4<f32> };
struct Camera { vp: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;


struct EdgeIn { @location(0) pos: vec3<f32> };


@vertex
fn vs_main(in: EdgeIn) -> VsOut {
var out: VsOut;
out.pos = camera.vp * vec4<f32>(in.pos, 1.0);
return out;
}


@fragment
fn fs_main() -> @location(0) vec4<f32> {
return vec4<f32>(0.55, 0.62, 0.75, 1.0);
}