struct InputData {
  originx: f32,
  originy: f32,
  endpointx: f32,
  endpointy: f32,
  left: f32,
  right: f32,
  along: f32,
  back: f32,
}

struct Out {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

struct CameraUniform {
    world_to_render: mat3x3<f32>,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> input: InputData;

@group(2) @binding(0)
var tex: texture_2d<f32>;
@group(2) @binding(1)
var sampler: sampler;

@vertex
fn vs_main(
  @builtin(vertex_index) vertex_index: u32,
) -> Out {
  let origin = vec3<f32>(input.originx, input.originy, 1);
  let end = vec3<f32>(input.endpointx, input.endpointy, 1);
  var along = vec3<f32>(1, 0, 0);
  if (!all(origin == end)) {
    along = normalize(end - origin);
  }
  var worldspace: vec3<f32>;
  var uv: vec2<f32> = vec2<f32>(0, 0);
  var perpendicular = vec3<f32>(-along.y, along.x, 0);
  switch vertex_index {
    case 0, 3: {
      worldspace = origin + perpendicular * input.left - along * input.back;
      uv = vec2<f32>(0, 0);
    }
    case 1, 5, default: {
      worldspace = origin - perpendicular * input.right + along * input.along;
      uv = vec2<f32>(1, 1);
    }
    case 2: {
      worldspace = origin - perpendicular * input.right - along * input.back;
      uv = vec2<f32>(0, 1);
    }
    case 4: {
      worldspace = origin + perpendicular * input.left + along * input.along;
      uv = vec2<f32>(1, 0);
    }
  }
  worldspace.z = 1;
  let renderspace = camera.world_to_render * worldspace;
  var ret: Out;
  ret.position = vec4<f32>(renderspace.x, renderspace.y, 1, 1);
  ret.uv = uv;
  return ret;
}

@fragment
fn fs_main(in: Out) -> @location(0) vec4<f32> {
  return textureSample(tex, sampler, in.uv);
}
