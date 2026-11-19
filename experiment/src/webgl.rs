use glam::Mat4;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

use crate::scene::{self, Mesh, Vertex};
use crate::world::World;

use super::{ACTOR_VERTICES, camera, canvas_size};

const VERTEX: &str = include_str!("tide.vert");
const FRAGMENT: &str = include_str!("tide.frag");

struct VertexStream {
    buffer: WebGlBuffer,
    layout: WebGlVertexArrayObject,
}

impl VertexStream {
    fn new(gl: &Gl) -> Result<Self, JsValue> {
        let buffer = gl
            .create_buffer()
            .ok_or("Could not allocate vertex buffer")?;
        let layout = gl
            .create_vertex_array()
            .ok_or("Could not allocate vertex layout")?;
        gl.bind_vertex_array(Some(&layout));
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&buffer));
        let stride = size_of::<Vertex>() as i32;
        for (attribute, count, offset) in [(0, 3, 0), (1, 3, 12), (2, 1, 24)] {
            gl.enable_vertex_attrib_array(attribute);
            gl.vertex_attrib_pointer_with_i32(attribute, count, Gl::FLOAT, false, stride, offset);
        }
        Ok(Self { buffer, layout })
    }
}

pub(super) struct Graphics {
    canvas: HtmlCanvasElement,
    gl: Gl,
    view: WebGlUniformLocation,
    eye_time: WebGlUniformLocation,
    harbor: VertexStream,
    harbor_len: i32,
    actors: VertexStream,
}

impl Graphics {
    pub(super) fn new(canvas: HtmlCanvasElement) -> Result<Self, JsValue> {
        let gl = canvas
            .get_context("webgl2")?
            .ok_or("WebGL 2 is unavailable")?
            .dyn_into::<Gl>()?;
        let program = program(&gl)?;
        let view = gl
            .get_uniform_location(&program, "view_projection")
            .ok_or("Missing camera matrix")?;
        let eye_time = gl
            .get_uniform_location(&program, "eye_time")
            .ok_or("Missing camera position")?;
        gl.use_program(Some(&program));
        gl.enable(Gl::DEPTH_TEST);
        gl.depth_func(Gl::LEQUAL);
        gl.clear_color(0.54, 0.66, 0.66, 1.0);

        let mesh = scene::harbor();
        let harbor = VertexStream::new(&gl)?;
        gl.buffer_data_with_u8_array(
            Gl::ARRAY_BUFFER,
            bytemuck::cast_slice(&mesh.0),
            Gl::STATIC_DRAW,
        );
        let actors = VertexStream::new(&gl)?;
        gl.buffer_data_with_i32(
            Gl::ARRAY_BUFFER,
            (ACTOR_VERTICES * size_of::<Vertex>()) as i32,
            Gl::DYNAMIC_DRAW,
        );

        Ok(Self {
            canvas,
            gl,
            view,
            eye_time,
            harbor,
            harbor_len: mesh.0.len() as i32,
            actors,
        })
    }

    pub(super) fn resize(&mut self, dpr: f64) {
        let (width, height) = canvas_size(&self.canvas, dpr);
        if self.canvas.width() != width || self.canvas.height() != height {
            self.canvas.set_width(width);
            self.canvas.set_height(height);
            self.gl.viewport(0, 0, width as i32, height as i32);
        }
    }

    pub(super) fn draw(&self, world: &World, actors: &Mesh, seconds: f32) -> Mat4 {
        let (view, camera) = camera(
            world,
            seconds,
            self.canvas.width(),
            self.canvas.height(),
            false,
        );
        let gl = &self.gl;
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        gl.uniform_matrix4fv_with_f32_array(
            Some(&self.view),
            false,
            bytemuck::cast_slice(&camera.view_projection),
        );
        gl.uniform4fv_with_f32_array(Some(&self.eye_time), &camera.eye_time);
        gl.bind_vertex_array(Some(&self.harbor.layout));
        gl.draw_arrays(Gl::TRIANGLES, 0, self.harbor_len);
        gl.bind_vertex_array(Some(&self.actors.layout));
        gl.bind_buffer(Gl::ARRAY_BUFFER, Some(&self.actors.buffer));
        gl.buffer_sub_data_with_i32_and_u8_array(
            Gl::ARRAY_BUFFER,
            0,
            bytemuck::cast_slice(&actors.0),
        );
        gl.draw_arrays(Gl::TRIANGLES, 0, actors.0.len() as i32);
        view
    }
}

fn shader(gl: &Gl, stage: u32, source: &str) -> Result<WebGlShader, JsValue> {
    let shader = gl.create_shader(stage).ok_or("Could not allocate shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl.get_shader_parameter(&shader, Gl::COMPILE_STATUS) == JsValue::TRUE {
        Ok(shader)
    } else {
        Err(JsValue::from_str(
            &gl.get_shader_info_log(&shader).unwrap_or_default(),
        ))
    }
}

fn program(gl: &Gl) -> Result<WebGlProgram, JsValue> {
    let program = gl
        .create_program()
        .ok_or("Could not allocate shader program")?;
    let vertex = shader(gl, Gl::VERTEX_SHADER, VERTEX)?;
    let fragment = shader(gl, Gl::FRAGMENT_SHADER, FRAGMENT)?;
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    if gl.get_program_parameter(&program, Gl::LINK_STATUS) == JsValue::TRUE {
        Ok(program)
    } else {
        Err(JsValue::from_str(
            &gl.get_program_info_log(&program).unwrap_or_default(),
        ))
    }
}
