#![feature(non_ascii_idents)]
use glam::Mat4;
use log;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::console;
use web_sys::*;
use web_sys::{
    window, HtmlCanvasElement, HtmlElement, MouseEvent, WebGlProgram,
    WebGlRenderingContext, WebGlShader, Window,
};

/**
 * The controller
 */
#[wasm_bindgen]
pub struct App {
    plot: Plot,
    view: View,
    canvas: HtmlCanvasElement,
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub fn new() -> App {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document.get_element_by_id("canvas").unwrap();
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("error casting canvas");

        let gl = canvas
            .get_context("webgl")
            .expect("error getting context webgl")
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .expect("error casting to webglrenderingcontext");
        let plot = Plot::new(Box::new(|x, y| 0.0), Box::new(|x, y| (0.0, 0.0)));
        let view = View::new(gl).expect("error creating view");

        App { plot, view, canvas }
    }

    pub fn update(&self) -> String {
        "boi".to_string()
    }

    pub fn render(&self) {
        let canvas_width = self.canvas.width();
        let canvas_height = self.canvas.height();
        let model = self.plot.gen_model(20);

        self.view
            .render(&model, canvas_width, canvas_height)
            .expect("error rendering view");
    }
}

/**
 * The view
 */
struct View {
    gl: WebGlRenderingContext,
    program: WebGlProgram,
    frustum: Frustum,
    world: World,
}

impl View {
    fn new(gl: WebGlRenderingContext) -> Result<View, JsValue> {
        let fov_y = 45.0;
        let front = 0.2;
        let back = 128.0;
        let α = std::f32::consts::PI * 5.0 / 8.0;
        let β = std::f32::consts::PI;
        let γ = std::f32::consts::PI;
        let zoom = 0.8f32;
        let xtrans = 0.0f32;
        let ytrans = 0.0f32;
        let ztrans = -3.0f32;

        let frustum = Frustum { fov_y, front, back };
        let world = World {
            roll: α,
            pitch: β,
            yaw: γ,
            zoom,
            xtrans,
            ytrans,
            ztrans,
        };
        let vert_shader = View::compile_shader(
            &gl,
            WebGlRenderingContext::VERTEX_SHADER,
            include_str!("shaders/vertex.glsl"),
        )?;
        let frag_shader = View::compile_shader(
            &gl,
            WebGlRenderingContext::FRAGMENT_SHADER,
            include_str!("shaders/fragment.glsl"),
        )?;

        let program = View::link_program(&gl, &vert_shader, &frag_shader)?;

        Ok(View {
            gl,
            program,
            frustum,
            world,
        })
    }

    fn render(
        &self,
        model: &Model,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Result<(), JsValue> {
        let gl = &self.gl;
        let program = &self.program;
        log::info!("render");
        let vertices = &model.vertices;
        let indices = &model.indices;
        let pm = self
            .frustum
            .gen_projection_matrix(canvas_width, canvas_height);
        let wm = self.world.gen_world_matrix();
        let buffer = gl.create_buffer().ok_or("failed to create buffer")?;

        gl.use_program(Some(program));

        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&buffer));

        // Note that `Float32Array::view` is somewhat dangerous (hence the
        // `unsafe`!). This is creating a raw view into our module's
        // `WebAssembly.Memory` buffer, but if we allocate more pages for ourself
        // (aka do a memory allocation in Rust) it'll cause the buffer to change,
        // causing the `Float32Array` to be invalid.
        //
        // As a result, after `Float32Array::view` we have to be very careful not to
        // do any memory allocations before it's dropped.
        unsafe {
            let vert_array = js_sys::Float32Array::view(&vertices);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        let buffer = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(
            WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
            Some(&buffer),
        );

        unsafe {
            let indices_array = js_sys::Uint16Array::view(&indices);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
                &indices_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        gl.enable(WebGlRenderingContext::DEPTH_TEST);

        let pm_loc = gl.get_uniform_location(&program, "pm");
        gl.uniform_matrix4fv_with_f32_array(
            pm_loc.as_ref(),
            false,
            pm.as_ref(),
        );

        let wm_loc = gl.get_uniform_location(&program, "wm");
        gl.uniform_matrix4fv_with_f32_array(
            wm_loc.as_ref(),
            false,
            wm.as_ref(),
        );

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(
            0,
            3,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        gl.clear_color(0.0, 0.0, 1.0, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.draw_elements_with_i32(
            WebGlRenderingContext::TRIANGLES,
            indices.len() as i32,
            WebGlRenderingContext::UNSIGNED_SHORT,
            0,
        );

        gl.disable_vertex_attrib_array(0);

        Ok(())
    }

    fn compile_shader(
        context: &WebGlRenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, String> {
        let shader = context
            .create_shader(shader_type)
            .ok_or_else(|| String::from("Unable to create shader object"))?;
        context.shader_source(&shader, source);
        context.compile_shader(&shader);

        if context
            .get_shader_parameter(
                &shader,
                WebGlRenderingContext::COMPILE_STATUS,
            )
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(context.get_shader_info_log(&shader).unwrap_or_else(|| {
                String::from("Unknown error creating shader")
            }))
        }
    }

    fn link_program(
        context: &WebGlRenderingContext,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
    ) -> Result<WebGlProgram, String> {
        let program = context
            .create_program()
            .ok_or_else(|| String::from("Unable to create shader object"))?;

        context.attach_shader(&program, vert_shader);
        context.attach_shader(&program, frag_shader);
        context.link_program(&program);

        if context
            .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            Err(context.get_program_info_log(&program).unwrap_or_else(|| {
                String::from("Unknown error creating program object")
            }))
        }
    }
}

struct World {
    ///Rotation around the x-axis
    roll: f32,

    ///Rotation around the y-axis
    pitch: f32,

    ///Rotation around the z-axis
    yaw: f32,

    zoom: f32,
    xtrans: f32,
    ytrans: f32,
    ztrans: f32,
}

impl World {
    pub fn gen_world_matrix(&self) -> Mat4 {
        let zoom = self.zoom;
        let xtrans = self.xtrans;
        let ytrans = self.ytrans;
        let ztrans = self.ztrans;

        use glam::{Quat, Vec3};

        let scale = Vec3::new(zoom, zoom, zoom);

        let rot_x = Quat::from_rotation_x(self.roll);
        let rot_y = Quat::from_rotation_y(self.pitch);
        let rot_z = Quat::from_rotation_z(self.yaw);

        let rot = rot_x * rot_y * rot_z;
        let trans = Vec3::new(xtrans, ytrans, ztrans);

        Mat4::from_scale_rotation_translation(scale, rot, trans)
    }
}

struct Frustum {
    fov_y: f32,
    front: f32,
    back: f32,
}

impl Frustum {
    pub fn gen_projection_matrix(
        &self,
        canvas_width: u32,
        canvas_height: u32,
    ) -> Mat4 {
        let deg2rad = std::f32::consts::PI / 180.0;

        let ratio: f32 = canvas_width as f32 / canvas_height as f32;

        Mat4::perspective_rh_gl(
            self.fov_y * deg2rad,
            ratio,
            self.front,
            self.back,
        )
    }
}

struct Model {
    vertices: Vec<f32>,
    indices: Vec<u16>,
    normals: Vec<f32>,
}

/**
 * The model
 */
struct Plot {
    equation: Box<Fn(f32, f32) -> f32>,
    gradient: Box<Fn(f32, f32) -> (f32, f32)>,
}

impl Plot {
    pub fn new(
        equation: Box<Fn(f32, f32) -> f32>,
        gradient: Box<Fn(f32, f32) -> (f32, f32)>,
    ) -> Plot {
        Plot { equation, gradient }
    }

    pub fn gen_model(&self, size: u16) -> Model {
        let unit_square = (0..size * size).map(|i| {
            let (x, y) = (i % size, i / size);
            let (x, y) =
                (x as f32 / (size - 1) as f32, y as f32 / (size - 1) as f32);
            (-1.0 + 2.0 * x, 1.0 - 2.0 * y)
        });

        let f = self.equation.as_ref();
        let del = self.gradient.as_ref();
        let (vertices, normals) = unit_square
            .flat_map(|(x, y)| {
                let (df_dx, df_dy) = del(x, y);
                vec![(x, -df_dx), (y, -df_dy), (f(x, y), 1.0)]
            })
            .unzip();

        let indices: Vec<u16> = (0..((size - 1) * (size - 1)))
            .map(|i| (i % (size - 1), i / (size - 1)))
            .flat_map(|(x, y)| {
                let top_left = y * size + x;
                let top_right = top_left + 1;
                let btm_left = (y + 1) * size + x;
                let btm_right = btm_left + 1;
                vec![
                    (top_left, btm_left, top_right),
                    (top_right, btm_left, btm_right),
                ]
            })
            .flat_map(|t| vec![t.0, t.1, t.2])
            .collect();

        Model {
            vertices,
            indices,
            normals,
        }
    }
}
