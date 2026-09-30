use std::{borrow::Cow, thread};

use eframe::{egui, wgpu};

use bytemuck;
use modstreams_core::{ModstreamsClient, Packet};

struct ShaderInput {
    name: String,
    value: f32,
    binding: u32,
    buffer: wgpu::Buffer,
}

impl ShaderInput {
    pub fn new(device: &wgpu::Device, name: String, binding: u32) -> ShaderInput {
        let buffer = device.create_buffer(&wgpu::wgt::BufferDescriptor {
            label: Some(&name),
            size: size_of::<f32>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            name,
            value: 0.,
            binding,
            buffer,
        }
    }

    pub fn create_bind_group_layout_entry(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            visibility: wgpu::ShaderStages::all(),
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<f32>() as u64),
            },
            count: None,
        }
    }

    pub fn create_bind_group_entry(&self) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding: self.binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.buffer,
                offset: 0,
                size: wgpu::BufferSize::new(size_of::<f32>() as u64),
            }),
        }
    }

    pub fn update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(&self.value));
    }
}

fn run_read_thread(
    ctx: egui::Context,
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::TextureView,
) {
    let shader_src = include_str!("../shader.wgsl");

    let module = wgpu::naga::front::wgsl::parse_str(shader_src).unwrap();
    let mut inputs = Vec::new();
    for (_, global) in module.global_variables.iter() {
        if let Some(name) = &global.name {
            if let Some(binding) = global.binding {
                inputs.push(ShaderInput::new(&device, name.clone(), binding.binding));
            }
        }
    }

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(shader_src)),
    });

    let mut bind_group_layout_entries = Vec::new();
    for input in &inputs {
        bind_group_layout_entries.push(input.create_bind_group_layout_entry());
    }

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &bind_group_layout_entries,
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let mut bind_group_entries = Vec::new();
    for input in &inputs {
        bind_group_entries.push(input.create_bind_group_entry());
    }

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &bind_group_layout,
        entries: &bind_group_entries,
    });

    let mut client = ModstreamsClient::new(7460);
    for input in &inputs {
        client.subscribe(&input.name).unwrap();
    }
    loop {
        let packet = client.read().unwrap();
        let mut updated = false;
        if let Packet::Message { channel, content } = packet {
            for input in &mut inputs {
                if input.name == channel {
                    if let Ok(s) = str::from_utf8(&content) {
                        if let Ok(v) = s.parse() {
                            input.value = v;
                            input.update(&queue);
                            updated = true;
                        }
                    }
                }
            }
        }

        if !updated {
            continue;
        }

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&render_pipeline);
            render_pass.set_bind_group(0, &bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        ctx.request_repaint();
    }
}

struct DisplayApp {
    egui_texture: egui::TextureId,
}

impl DisplayApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu_state = cc.wgpu_render_state.clone().unwrap();
        let renderer = wgpu_state.renderer.clone();
        let device = wgpu_state.device;
        let queue = wgpu_state.queue;

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 800,
                height: 600,
                ..Default::default()
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let egui_texture = renderer.write().register_native_texture(
            &device,
            &texture_view,
            wgpu::FilterMode::Linear,
        );

        let ctx = cc.egui_ctx.clone();

        thread::spawn(move || run_read_thread(ctx, device, queue, texture_view));

        Self { egui_texture }
    }
}

impl eframe::App for DisplayApp {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add(egui::Image::new((
                self.egui_texture,
                egui::Vec2::new(800., 600.),
            )));
        });
    }
}

fn main() {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Display",
        native_options,
        Box::new(|cc| Ok(Box::new(DisplayApp::new(cc)))),
    )
    .unwrap();
}
