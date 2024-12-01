@vertex
fn vertexMain(@location(0) pos: vec3f) -> @builtin(position) vec4f {
  return vec4f(pos, 1);
}

@fragment
fn fragmentMain() -> @location(0) vec4f {
    return vec4f(1, 0, 0, 1);
}
