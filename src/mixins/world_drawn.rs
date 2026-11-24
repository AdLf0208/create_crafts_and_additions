fn wire_world_drawn(world: &mut WorldDrawn<'_>) {
  // A pipeline is expensive to build (it compiles the shader), so it is kept between frames
  // and rebuilt only when the frame's target format or depth choice changes.
  struct Cache {
    format: wgpu::TextureFormat,
    depth: bool,
    pipeline: wgpu::RenderPipeline,
  }
  static CACHE: ::std::sync::Mutex<::core::option::Option<Cache>> =
    ::std::sync::Mutex::new(::core::option::Option::None);
  const SHADER: &str = "\
struct Input {
  @location(0) clip: vec4<f32>,
  @location(1) rgba: vec4<f32>,
};
struct Output {
  @builtin(position) position: vec4<f32>,
  @location(0) rgba: vec4<f32>,
};
@vertex
fn vs(input: Input) -> Output {
  var out: Output;
  out.position = input.clip;
  out.rgba = input.rgba;
  return out;
}
@fragment
fn fs(input: Output) -> @location(0) vec4<f32> {
  return input.rgba;
}
";
  const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];

  let wires = createaddition::render::wires();
  let held = createaddition::render::held();
  let gauges = createaddition::render::gauges();
  if wires.is_empty() && held.is_empty() && gauges.is_empty() {
    return;
  }
  let camera = [world.camera.x, world.camera.y, world.camera.z];
  let view_proj = world.view_proj.to_cols_array();
  // A held spool's preview is drawn by the same `push_wire` as a laid wire, so it shares this
  // vertex buffer and draw call (and the lightmap carried in 0.2.14) — no second mixin row.
  let mut vertices = createaddition::render::frame(&wires, camera, &view_proj);
  vertices.extend(createaddition::render::held_frame(&held, camera, &view_proj));
  // The accumulator's needle uses this same pipeline and vertex layout — a second draw of the
  // same shader, not a second render path. Both are separate buffers so either can be absent.
  let gauge_vertices = createaddition::render::gauge_frame(&gauges, camera, &view_proj);
  if vertices.is_empty() && gauge_vertices.is_empty() {
    return;
  }
  // The world's depth is attached only when it is the frame's own size: a pass's depth
  // attachment must cover its colour attachment, and a lower-resolution world has one that
  // does not. Without it the wire is drawn over the world instead of behind walls.
  let depth = world.depth_size == world.size;

  let mut cache = CACHE
    .lock()
    .expect("the createaddition wire pipeline lock is held for one frame");
  let stale = match cache.as_ref() {
    Some(held) => held.format != world.color_format || held.depth != depth,
    None => true,
  };
  if stale {
    let module = world
      .device
      .create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("createaddition:wire"),
        source: wgpu::ShaderSource::Wgsl(SHADER.into()),
      });
    let pipeline = world
      .device
      .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("createaddition:wire"),
        layout: None,
        vertex: wgpu::VertexState {
          module: &module,
          entry_point: Some("vs"),
          compilation_options: wgpu::PipelineCompilationOptions::default(),
          buffers: &[Some(wgpu::VertexBufferLayout {
            array_stride: 32,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
          })],
        },
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleList,
          cull_mode: None,
          ..wgpu::PrimitiveState::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
          format: wgpu::TextureFormat::Depth32Float,
          // Reversed depth: near is 1, the sky 0. The wire never writes it.
          depth_write_enabled: Some(false),
          depth_compare: Some(wgpu::CompareFunction::Greater),
          stencil: wgpu::StencilState::default(),
          bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
          module: &module,
          entry_point: Some("fs"),
          compilation_options: wgpu::PipelineCompilationOptions::default(),
          targets: &[Some(wgpu::ColorTargetState {
            format: world.color_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
          })],
        }),
        multiview_mask: None,
        cache: None,
      });
    *cache = Some(Cache {
      format: world.color_format,
      depth,
      pipeline,
    });
  }
  let pipeline = &cache
    .as_ref()
    .expect("the wire pipeline is built above")
    .pipeline;

  let wire_bytes = createaddition::render::vertex_bytes(&vertices);
  let wire_buffer = world.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("createaddition:wires"),
    size: wire_bytes.len().max(1) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  if !wire_bytes.is_empty() {
    world.queue.write_buffer(&wire_buffer, 0, &wire_bytes);
  }
  let gauge_bytes = createaddition::render::vertex_bytes(&gauge_vertices);
  let gauge_buffer = world.device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("createaddition:gauges"),
    size: gauge_bytes.len().max(1) as u64,
    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  });
  if !gauge_bytes.is_empty() {
    world.queue.write_buffer(&gauge_buffer, 0, &gauge_bytes);
  }

  {
    let mut pass = world
      .encoder
      .begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("createaddition:wires"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: world.color_view,
          depth_slice: None,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Load,
            store: wgpu::StoreOp::Store,
          },
        })],
        depth_stencil_attachment: if depth {
          Some(wgpu::RenderPassDepthStencilAttachment {
            view: world.depth,
            depth_ops: Some(wgpu::Operations {
              load: wgpu::LoadOp::Load,
              store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
          })
        } else {
          None
        },
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
      });
    pass.set_pipeline(pipeline);
    if !vertices.is_empty() {
      pass.set_vertex_buffer(0, wire_buffer.slice(..));
      pass.draw(0..vertices.len() as u32, 0..1);
    }
    if !gauge_vertices.is_empty() {
      pass.set_vertex_buffer(0, gauge_buffer.slice(..));
      pass.draw(0..gauge_vertices.len() as u32, 0..1);
    }
  }
}
