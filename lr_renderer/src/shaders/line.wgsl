struct Line {
  x1: f32,
  y1: f32,
  x2: f32,
  y2: f32,
  width: f32,
}

struct CameraUniform {
    world_to_render: mat3x3<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> linebuf: array<Line>;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
  @location(1) length_total: f32,
  @location(2) radius: f32,
};

@vertex
fn vs_main(
  @builtin(vertex_index) vertex_index: u32,
  @builtin(instance_index) instance_index: u32,
) -> VertexOutput {
  let line = linebuf[instance_index];
  let x1 = line.x1;
  let y1 = line.y1;
  let x2 = line.x2;
  let y2 = line.y2;
  let width = line.width;
  let start: vec3<f32> = vec3<f32>(x1, y1, 1);
  let end: vec3<f32> = vec3<f32>(x2, y2, 1);
  let along: vec3<f32> = normalize(end - start) * width;
  let out: vec3<f32> = vec3<f32>(-along.y, along.x, 1);
  var worldspace: vec3<f32>;
  var uv: vec2<f32> = vec2<f32>(0, 0);
  let length_total = distance(start, end) + width * 2;
  switch vertex_index {
    case 0, 3: {
      worldspace = start + out - along;
      uv = vec2<f32>(0, 0);
    }
    case 1, 5, default: {
      worldspace = end - out + along;
      uv = vec2<f32>(length_total, width * 2);
    }
    case 2: {
      worldspace = start - out - along;
      uv = vec2<f32>(0, width * 2);
    }
    case 4: {
      worldspace = end + out + along;
      uv = vec2<f32>(length_total, 0);
    }
  }
  worldspace.z = 1;
  let renderspace = camera.world_to_render * worldspace;
  var ret: VertexOutput;
  ret.position = vec4<f32>(renderspace.x, renderspace.y, 1, 1);
  ret.uv = uv;
  ret.length_total = length_total;
  ret.radius = width;
  return ret;
}

@group(1) @binding(1)
var<uniform> color: vec4<f32>;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
  if (in.uv.x > in.radius && in.uv.x < in.length_total - in.radius) {
    return color;
  }
  var circle: vec2<f32>;
  if (in.uv.x <= in.radius) {
    circle = vec2<f32>(in.radius, in.radius);
  } else {
    circle = vec2<f32>(in.length_total - in.radius, in.radius);
  }

  if (distance(in.uv, circle) > in.radius) {
    discard;
  } else {
    return color;
  }
}
