pub fn get_cell_bind_groups(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    grid_uniform_buffer: &wgpu::Buffer,
    cell_state_storage_buffer: &[wgpu::Buffer; 2],
) -> [wgpu::BindGroup; 2] {
    [
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cell_state_storage_buffer[0].as_entire_binding(),
                },
            ],
            label: Some("Cell Bind group A"),
        }),
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: grid_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: cell_state_storage_buffer[1].as_entire_binding(),
                },
            ],
            label: Some("Cell Bind group B"),
        }),
    ]
}
