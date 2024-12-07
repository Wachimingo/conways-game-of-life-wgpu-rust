@compute @workgroup_size(1)
fn computeMain(@builtin(global_invocation_id) cell: vec3u) {}
