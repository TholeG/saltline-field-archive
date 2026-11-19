use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    rc::Rc,
};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};
use wasm_bindgen::{JsCast, JsValue, closure::Closure, prelude::wasm_bindgen};
use web_sys::{
    AudioContext, Document, Event, HtmlCanvasElement, HtmlElement, KeyboardEvent, PointerEvent,
    Window,
};
use wgpu::{
    Backends, Buffer, BufferUsages, Color, CompareFunction, CurrentSurfaceTexture,
    DepthStencilState, Device, DeviceDescriptor, Extent3d, FragmentState, InstanceDescriptor,
    Limits, LoadOp, Operations, PowerPreference, Queue, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPipeline,
    RenderPipelineDescriptor, RequestAdapterOptions, StoreOp, Surface, SurfaceConfiguration,
    Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor, VertexBufferLayout, VertexState, VertexStepMode, util::DeviceExt,
};

use crate::{
    scene::{self, Mesh, Vertex},
    world::{Controls, Stage, World},
};

#[path = "webgl.rs"]
mod webgl;

const DEPTH: TextureFormat = TextureFormat::Depth24Plus;
const ACTOR_VERTICES: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Camera {
    view_projection: [[f32; 4]; 4],
    eye_time: [f32; 4],
    settings: [f32; 4],
}

fn canvas_size(canvas: &HtmlCanvasElement, dpr: f64) -> (u32, u32) {
    let width = canvas.client_width().max(1) as f64;
    let height = canvas.client_height().max(1) as f64;
    let scale = dpr.min(1.5).min(1920.0 / width.max(height));
    (
        (width * scale).ceil().max(1.0) as u32,
        (height * scale).ceil().max(1.0) as u32,
    )
}

fn camera(world: &World, seconds: f32, width: u32, height: u32, srgb: bool) -> (Mat4, Camera) {
    let yaw = world.camera_angle;
    let focus = Vec3::new(world.player.x, 0.48, world.player.y)
        + Vec3::new(yaw.cos(), 0.0, -yaw.sin()) * 1.2;
    let eye = focus + Vec3::new(yaw.sin() * 11.6, 9.4, yaw.cos() * 11.6);
    let view_projection = glam::camera::rh::proj::directx::perspective(
        49.0_f32.to_radians(),
        width as f32 / height as f32,
        0.1,
        220.0,
    ) * glam::camera::rh::view::look_at_mat4(eye, focus, Vec3::Y);
    (
        view_projection,
        Camera {
            view_projection: view_projection.to_cols_array_2d(),
            eye_time: eye.extend(seconds).to_array(),
            settings: [if srgb { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0],
        },
    )
}

enum Renderer {
    Gpu(Box<Graphics>),
    Gl(webgl::Graphics),
}

impl Renderer {
    async fn new(canvas: HtmlCanvasElement, force_webgl: bool) -> Result<Self, JsValue> {
        if force_webgl {
            Ok(Self::Gl(webgl::Graphics::new(canvas)?))
        } else {
            Ok(Self::Gpu(Box::new(Graphics::new(canvas).await?)))
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Gpu(_) => "WebGPU",
            Self::Gl(_) => "WebGL 2",
        }
    }

    fn resize(&mut self, dpr: f64) {
        match self {
            Self::Gpu(renderer) => renderer.resize(dpr),
            Self::Gl(renderer) => renderer.resize(dpr),
        }
    }

    fn draw(&mut self, world: &World, actors: &mut Mesh, seconds: f32) -> Mat4 {
        scene::actors(actors, world, seconds);
        match self {
            Self::Gpu(renderer) => renderer.draw(world, actors, seconds),
            Self::Gl(renderer) => renderer.draw(world, actors, seconds),
        }
    }
}

struct Graphics {
    canvas: HtmlCanvasElement,
    surface: Surface<'static>,
    config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    pipeline: RenderPipeline,
    camera: Buffer,
    camera_bind: wgpu::BindGroup,
    harbor: Buffer,
    harbor_len: u32,
    actors: Buffer,
    depth: Texture,
}

impl Graphics {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let mut desc = InstanceDescriptor::new_without_display_handle();
        desc.backends = Backends::BROWSER_WEBGPU;
        let instance = wgpu::util::new_instance_with_webgpu_detection(desc).await;
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?;
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("Tidewatch"),
                required_limits: Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                ..Default::default()
            })
            .await
            .map_err(js_error)?;
        let Some(config) = surface.get_default_config(&adapter, 640, 360) else {
            return Err(JsValue::from_str("The browser cannot display a 3D canvas."));
        };
        let validation = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::include_wgsl!("tide.wgsl"));
        let attributes = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32];
        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Harbor color and depth"),
            layout: None,
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[Some(VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as u64,
                    step_mode: VertexStepMode::Vertex,
                    attributes: &attributes,
                })],
            },
            primitive: Default::default(),
            depth_stencil: Some(DepthStencilState {
                format: DEPTH,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::LessEqual),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(config.format.into())],
            }),
            multiview_mask: None,
            cache: None,
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera"),
            size: std::mem::size_of::<Camera>() as u64,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera binding"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let mesh = scene::harbor();
        let harbor = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Static harbor mesh"),
            contents: bytemuck::cast_slice(&mesh.0),
            usage: BufferUsages::VERTEX,
        });
        let actors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Avatar, gate and ferry"),
            size: (std::mem::size_of::<Vertex>() * ACTOR_VERTICES) as u64,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth = Self::depth(&device, &config);
        surface.configure(&device, &config);
        if let Some(error) = validation.pop().await {
            return Err(js_error(error));
        }
        Ok(Self {
            canvas,
            surface,
            config,
            device,
            queue,
            pipeline,
            camera,
            camera_bind,
            harbor,
            harbor_len: mesh.0.len() as u32,
            actors,
            depth,
        })
    }

    fn depth(device: &Device, config: &SurfaceConfiguration) -> Texture {
        device.create_texture(&TextureDescriptor {
            label: Some("Depth"),
            size: Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: DEPTH,
            usage: TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
    }

    fn resize(&mut self, dpr: f64) {
        let (width, height) = canvas_size(&self.canvas, dpr);
        if self.config.width == width
            && self.config.height == height
            && self.canvas.width() == width
            && self.canvas.height() == height
        {
            return;
        }
        self.canvas.set_width(width);
        self.canvas.set_height(height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth = Self::depth(&self.device, &self.config);
    }

    fn draw(&mut self, world: &World, actors: &Mesh, seconds: f32) -> Mat4 {
        let (view_projection, camera) = camera(
            world,
            seconds,
            self.config.width,
            self.config.height,
            self.config.format.is_srgb(),
        );
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return view_projection;
            }
            _ => return view_projection,
        };
        self.queue
            .write_buffer(&self.camera, 0, bytemuck::bytes_of(&camera));
        self.queue
            .write_buffer(&self.actors, 0, bytemuck::cast_slice(&actors.0));
        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let depth = self.depth.create_view(&TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Tidewatch"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.54,
                            g: 0.66,
                            b: 0.66,
                            a: 1.0,
                        }),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.camera_bind, &[]);
            pass.set_vertex_buffer(0, self.harbor.slice(..));
            pass.draw(0..self.harbor_len, 0..1);
            pass.set_vertex_buffer(0, self.actors.slice(..));
            pass.draw(0..actors.0.len() as u32, 0..1);
        }
        self.queue.submit([encoder.finish()]);
        self.queue.present(frame);
        view_projection
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn element(document: &Document, id: &str) -> Result<HtmlElement, JsValue> {
    document
        .get_element_by_id(id)
        .and_then(|node| node.dyn_into().ok())
        .ok_or_else(|| JsValue::from_str(&format!("Missing interface element {id}")))
}

#[derive(Default)]
struct Input {
    keys: HashSet<String>,
    orbit: f32,
}

impl Input {
    fn controls(&mut self, dt: f32) -> Controls {
        let key = |code: &str| self.keys.contains(code);
        let walk = Vec2::new(
            (key("KeyD") as i8 - key("KeyA") as i8) as f32,
            ((key("KeyW") || key("ArrowUp")) as i8 - (key("KeyS") || key("ArrowDown")) as i8)
                as f32,
        );
        let turn =
            (key("ArrowLeft") as i8 - key("ArrowRight") as i8) as f32 * 1.8 * dt + self.orbit;
        let interact = key("KeyE");
        self.orbit = 0.0;
        Controls {
            walk,
            turn,
            interact,
        }
    }
}

struct Interface {
    overlay: HtmlElement,
    start: HtmlElement,
    canvas: HtmlCanvasElement,
    mission_title: HtmlElement,
    mission_text: HtmlElement,
    progress: HtmlElement,
    marker: HtmlElement,
    marker_label: HtmlElement,
    state: Option<(Stage, bool, i8)>,
}

impl Interface {
    fn update(&mut self, world: &World, matrix: Mat4, started: bool) {
        let stage = world.stage;
        let station = stage.station();
        let near = world.near_station();
        let alignment = if stage == Stage::Signal && near {
            world.alignment()
        } else {
            0
        };
        let state = (stage, near, alignment);
        if Some(state) != self.state {
            let (title, text) = match state {
                (Stage::Power, true, _) => ("Restore shore power", "Hold E at the breaker."),
                (Stage::Power, false, _) => {
                    ("Reach the breaker", "Restore power to the harbor controls.")
                }
                (Stage::Gate, true, _) => {
                    ("Raise the gate", "Keep holding E until the gate lifts.")
                }
                (Stage::Gate, false, _) => {
                    ("Reach the winch", "Cross the wooden pier to the winch.")
                }
                (Stage::Signal, false, _) => (
                    "Reach the signal",
                    "The ferry is waiting for a clear light.",
                ),
                (Stage::Signal, true, -1) => ("Align the signal", "Hold E and turn right with D."),
                (Stage::Signal, true, 1) => ("Align the signal", "Hold E and turn left with A."),
                (Stage::Signal, true, _) => {
                    ("Keep the light centered", "Release A/D and keep holding E.")
                }
                (Stage::Voyage, _, _) => ("Ferry in transit", "Watch the ferry clear the gate."),
                (Stage::Complete, _, _) => ("Channel clear", "The ferry has cleared the harbor."),
            };
            self.mission_title.set_text_content(Some(title));
            self.mission_text.set_text_content(Some(text));
            self.progress.set_hidden(!near);
            if let Some((_, label)) = station {
                self.marker_label.set_text_content(Some(label));
            }
            self.state = Some(state);
        }
        if near {
            let _ = self
                .progress
                .style()
                .set_property("--progress", &format!("{:.4}", world.progress));
            let _ = self.progress.set_attribute(
                "aria-valuenow",
                &(world.progress * 100.0).round().to_string(),
            );
        }
        let show_marker = started && station.is_some();
        if self.marker.hidden() == show_marker {
            self.marker.set_hidden(!show_marker);
        }
        if let Some((point, _)) = station.filter(|_| started) {
            let clip = matrix * Vec3::new(point.x, 1.82, point.y).extend(1.0);
            let direction = if clip.w > 0.05 {
                Vec2::new(clip.x, -clip.y) / clip.w
            } else {
                Vec2::new(clip.x.signum(), 0.0) * 2.0
            };
            let target = direction.clamp(Vec2::new(-0.86, -0.66), Vec2::new(0.86, 0.72));
            let _ = self
                .marker
                .style()
                .set_property("left", &format!("{:.2}%", (target.x + 1.0) * 50.0));
            let _ = self
                .marker
                .style()
                .set_property("top", &format!("{:.2}%", (target.y + 1.0) * 50.0));
            let _ = self.marker.set_attribute(
                "data-offscreen",
                if (target - direction).length_squared() > 0.001 {
                    "true"
                } else {
                    "false"
                },
            );
            let _ = self.marker.style().set_property(
                "--direction",
                &format!("{:.1}deg", direction.y.atan2(direction.x).to_degrees()),
            );
        }
    }
}

fn install_input(
    window: &Window,
    canvas: &HtmlCanvasElement,
    input: &Rc<RefCell<Input>>,
    started: &Rc<Cell<bool>>,
) -> Result<(), JsValue> {
    let keyboard_input = input.clone();
    let active = started.clone();
    let keydown = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        let code = event.code();
        if !active.get()
            || event.meta_key()
            || event.ctrl_key()
            || event.alt_key()
            || !matches!(
                code.as_str(),
                "KeyW"
                    | "KeyA"
                    | "KeyS"
                    | "KeyD"
                    | "KeyE"
                    | "ArrowLeft"
                    | "ArrowRight"
                    | "ArrowUp"
                    | "ArrowDown"
            )
        {
            return;
        }
        event.prevent_default();
        keyboard_input.borrow_mut().keys.insert(code);
    });
    window.add_event_listener_with_callback("keydown", keydown.as_ref().unchecked_ref())?;
    keydown.forget();

    let keyboard_input = input.clone();
    let keyup = Closure::<dyn FnMut(KeyboardEvent)>::new(move |event: KeyboardEvent| {
        keyboard_input.borrow_mut().keys.remove(&event.code());
    });
    window.add_event_listener_with_callback("keyup", keyup.as_ref().unchecked_ref())?;
    keyup.forget();
    let keyboard_input = input.clone();
    let blur = Closure::<dyn FnMut(Event)>::new(move |_| {
        keyboard_input.borrow_mut().keys.clear();
    });
    window.add_event_listener_with_callback("blur", blur.as_ref().unchecked_ref())?;
    blur.forget();

    let active = started.clone();
    let target = canvas.clone();
    let down = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        if active.get() && event.button() == 0 {
            let _ = target.set_pointer_capture(event.pointer_id());
        }
    });
    canvas.add_event_listener_with_callback("pointerdown", down.as_ref().unchecked_ref())?;
    down.forget();

    let pointer_input = input.clone();
    let target = canvas.clone();
    let drag = Closure::<dyn FnMut(PointerEvent)>::new(move |event: PointerEvent| {
        if target.has_pointer_capture(event.pointer_id()) {
            pointer_input.borrow_mut().orbit -=
                (event.movement_x() as f32).clamp(-200.0, 200.0) * 0.006;
        }
    });
    canvas.add_event_listener_with_callback("pointermove", drag.as_ref().unchecked_ref())?;
    drag.forget();
    Ok(())
}

fn chime(audio: &AudioContext) {
    let now = audio.current_time();
    for (delay, frequency) in [(0.0, 392.0), (0.14, 523.25), (0.28, 659.25)] {
        let Ok(oscillator) = audio.create_oscillator() else {
            return;
        };
        let Ok(gain) = audio.create_gain() else {
            return;
        };
        let time = now + delay;
        let _ = oscillator.frequency().set_value_at_time(frequency, time);
        let _ = gain.gain().set_value_at_time(0.0, time);
        let _ = gain.gain().linear_ramp_to_value_at_time(0.06, time + 0.045);
        let _ = gain.gain().linear_ramp_to_value_at_time(0.0, time + 0.68);
        let _ = oscillator.connect_with_audio_node(&gain);
        let _ = gain.connect_with_audio_node(&audio.destination());
        let _ = oscillator.start_with_when(time);
        let _ = oscillator.stop_with_when(time + 0.72);
    }
}

type Animation = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

#[wasm_bindgen]
pub async fn start(canvas: HtmlCanvasElement, force_webgl: bool) -> Result<String, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("Missing browser window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Missing browser document"))?;
    let mut graphics = Renderer::new(canvas.clone(), force_webgl).await?;
    let backend = graphics.name().to_owned();
    let mut ui = Interface {
        overlay: element(&document, "welcome")?,
        start: element(&document, "enter-harbor")?,
        canvas: canvas.clone(),
        mission_title: element(&document, "mission-title")?,
        mission_text: element(&document, "mission-text")?,
        progress: element(&document, "progress")?,
        marker: element(&document, "marker")?,
        marker_label: element(&document, "marker-label")?,
        state: None,
    };
    let started = Rc::new(Cell::new(false));
    let input = Rc::new(RefCell::new(Input::default()));
    let audio: Rc<RefCell<Option<AudioContext>>> = Rc::default();
    install_input(&window, &canvas, &input, &started)?;
    {
        let pressed = started.clone();
        let overlay = ui.overlay.clone();
        let canvas = ui.canvas.clone();
        let audio = audio.clone();
        let activate = Closure::<dyn FnMut(Event)>::new(move |_| {
            if pressed.replace(true) {
                return;
            }
            overlay.set_hidden(true);
            let _ = canvas.focus();
            if let Ok(context) = AudioContext::new() {
                let _ = context.resume();
                audio.replace(Some(context));
            }
        });
        ui.start
            .add_event_listener_with_callback("click", activate.as_ref().unchecked_ref())?;
        activate.forget();
    }
    let mut world = World::new();
    let mut actors = Mesh(Vec::with_capacity(ACTOR_VERTICES));
    graphics.resize(window.device_pixel_ratio());
    graphics.draw(&world, &mut actors, 0.0);
    let animation: Animation = Rc::default();
    let repeated = animation.clone();
    let mut last = None;
    let mut seconds = 0.0;
    animation.replace(Some(Closure::<dyn FnMut(f64)>::new(move |now: f64| {
        let dt = last
            .map(|time: f64| ((now - time) / 1000.0).clamp(0.0, 0.05) as f32)
            .unwrap_or(0.0);
        last = Some(now);
        seconds += dt;
        if started.get() {
            let controls = input.borrow_mut().controls(dt);
            if world.update(dt, controls)
                && let Some(context) = audio.borrow().as_ref()
            {
                chime(context);
            }
        }
        graphics.resize(window.device_pixel_ratio());
        let view = graphics.draw(&world, &mut actors, seconds);
        ui.update(&world, view, started.get());
        if let Some(callback) = repeated.borrow().as_ref() {
            let _ = window.request_animation_frame(callback.as_ref().unchecked_ref());
        }
    })));
    web_sys::window().unwrap().request_animation_frame(
        animation
            .borrow()
            .as_ref()
            .unwrap()
            .as_ref()
            .unchecked_ref(),
    )?;
    Ok(backend)
}
