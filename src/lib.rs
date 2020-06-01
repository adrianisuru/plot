#![feature(non_ascii_idents)]
use glam::Mat4;
use log;
use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, WebGlProgram, WebGlRenderingContext, WebGlShader,
};

mod weblogger;
use weblogger::WebLogger;

static WEB_LOGGER: WebLogger = WebLogger;

#[wasm_bindgen]
pub fn cring() -> String {
    "cringe".to_string()
}

/**
 * The controller
 */
#[wasm_bindgen]
pub struct App {
    plot: Plot,
    view: View,
    canvas: HtmlCanvasElement,
    mouse_down: Rc<Cell<bool>>,
    mouse_pos: Rc<Cell<(i32, i32)>>,
    last_mouse_down_pos: Rc<Cell<(i32, i32)>>,
}

/**
 * More rust friendly version of part of `web_sys::EventTarget`'s api
 * Here `listener` is a rust closure instead of a `wasm_bindgen::closure::Closure`
 */
trait EventTarget<C>
where
    C: Fn(web_sys::MouseEvent),
{
    fn add_event_listener_with_callback(
        &self,
        type_: &str,
        listener: C,
    ) -> Result<(), JsValue>;
}

impl<C> EventTarget<C> for HtmlCanvasElement
where
    C: Fn(web_sys::MouseEvent) + 'static,
{
    fn add_event_listener_with_callback(
        &self,
        type_: &str,
        listener: C,
    ) -> Result<(), JsValue> {
        let handler = Closure::wrap(Box::new(listener) as Box<dyn FnMut(_)>);
        {
            //need to use web_sys::EventTarget.add_event_listener_with_callback() so we get self as
            //web_sys::EventTarget first
            let et: &web_sys::EventTarget = self.as_ref();
            et.add_event_listener_with_callback(
                type_,
                handler.as_ref().unchecked_ref(),
            )?;
        }
        handler.forget();
        Ok(())
    }
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<App, JsValue> {
        log::set_logger(&WEB_LOGGER).unwrap();
        log::set_max_level(log::LevelFilter::Info);
        log::info!("logger active!");
        let document = web_sys::window()
            .expect("cannot find window")
            .document()
            .expect("cannot find document");
        let canvas = document
            .get_element_by_id("canvas")
            .expect("cannot find canvas");
        let canvas: web_sys::HtmlCanvasElement = canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("error casting canvas");

        let gl = canvas
            .get_context("webgl")
            .expect("error getting context webgl")
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()
            .expect("error casting to webglrenderingcontext");

        let c = 5.0;
        let f = move |x: f32, y: f32| (c * x).sin() * (c * y).cos() / c;
        let g = move |x: f32, y: f32| {
            (
                (c * x).cos() * (c * y).cos(),
                -(c * x).sin() * (c * y).sin(),
            )
        };
        let plot = Plot::new(f, g);

        let view = View::new(gl).expect("error creating view");

        let mouse_down = false;
        let mouse_down = Rc::new(Cell::new(mouse_down));

        let pos = (0, 0);
        let mouse_pos = Rc::new(Cell::new(pos));
        let last_mouse_down_pos = Rc::new(Cell::new(pos));

        use web_sys::MouseEvent;

        {
            let mouse_down = mouse_down.clone();
            let last_mouse_down_pos = last_mouse_down_pos.clone();
            canvas.add_event_listener_with_callback(
                "mousedown",
                move |event: MouseEvent| {
                    let pos = (event.x(), event.y());
                    log::info!("mousedown");
                    mouse_down.set(true);
                    last_mouse_down_pos.set(pos);
                },
            )?;
        }
        {
            let mouse_down = mouse_down.clone();
            canvas.add_event_listener_with_callback(
                "mouseup",
                move |_: MouseEvent| {
                    log::info!("mouseup");
                    mouse_down.set(false);
                },
            )?;
        }
        {
            let mouse_down = mouse_down.clone();
            canvas.add_event_listener_with_callback(
                "mouseleave",
                move |_: MouseEvent| {
                    log::info!("mouseleave");
                    mouse_down.set(false);
                },
            )?;
        }
        {
            let mouse_pos = mouse_pos.clone();
            canvas.add_event_listener_with_callback(
                "mousemove",
                move |event: MouseEvent| {
                    let pos = (event.x(), event.y());
                    log::info!("{:?}", pos);
                    mouse_pos.set(pos);
                },
            )?;
        }

        Ok(App {
            plot,
            view,
            canvas,
            mouse_down,
            mouse_pos,
            last_mouse_down_pos,
        })
    }

    pub fn update(&mut self) {
        self.update_tilt();
    }

    fn update_tilt(&mut self) {
        let mouse_down = self.mouse_down.get();
        let (x, y) = self.mouse_pos.get();

        if mouse_down {
            let canvas_width = self.canvas.width();
            let canvas_height = self.canvas.height();

            let (x0, y0) = self.last_mouse_down_pos.get();
            let dx = (x0 - x) as f32 / canvas_width as f32;
            let dy = (y0 - y) as f32 / canvas_height as f32;

            let roll = self.view.world.roll;
            let pitch = self.view.world.pitch;
            //let yaw = self.view.world.yaw;

            self.view.world.pitch = pitch - dx.atan();
            self.view.world.roll = roll - dy.atan();

            self.last_mouse_down_pos.set((x, y));
        }
    }

    pub fn render(&self) {
        let canvas_width = self.canvas.width();
        let canvas_height = self.canvas.height();
        let model = self.plot.gen_model(30);

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
        let roll = std::f32::consts::FRAC_PI_8;
        let pitch = 0.0;
        let yaw = 0.0;
        let zoom = 0.8f32;
        let xtrans = 0.0f32;
        let ytrans = 0.0f32;
        let ztrans = -3.0f32;

        let frustum = Frustum { fov_y, front, back };
        let world = World {
            roll,
            pitch,
            yaw,
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
        //log::info!("render");
        let vertices = &model.vertices;
        let indices = &model.indices;
        let normals = &model.normals;
        let pm = self
            .frustum
            .gen_projection_matrix(canvas_width, canvas_height);
        let wm = self.world.gen_world_matrix();
        let nm = self.world.gen_normal_matrix();

        gl.use_program(Some(program));

        let pos_buff = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&pos_buff));

        unsafe {
            let vert_array = js_sys::Float32Array::view(&vertices);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &vert_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        let idx_buff = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(
            WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
            Some(&idx_buff),
        );

        unsafe {
            let indices_array = js_sys::Uint16Array::view(&indices);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
                &indices_array,
                WebGlRenderingContext::STATIC_DRAW,
            );
        }

        let norm_buff = gl.create_buffer().ok_or("failed to create buffer")?;
        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&norm_buff));

        unsafe {
            let norm_array = js_sys::Float32Array::view(&normals);

            gl.buffer_data_with_array_buffer_view(
                WebGlRenderingContext::ARRAY_BUFFER,
                &norm_array,
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

        let nm_loc = gl.get_uniform_location(&program, "nm");
        gl.uniform_matrix4fv_with_f32_array(
            nm_loc.as_ref(),
            false,
            nm.as_ref(),
        );

        let pos_loc = gl.get_attrib_location(&program, "position") as u32;
        //let pos_loc = 0;

        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&pos_buff));
        gl.enable_vertex_attrib_array(pos_loc);
        gl.vertex_attrib_pointer_with_i32(
            pos_loc,
            3,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        let norm_loc = gl.get_attrib_location(&program, "normal") as u32;
        //let norm_loc = 1;

        gl.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&norm_buff));
        gl.enable_vertex_attrib_array(norm_loc);
        gl.vertex_attrib_pointer_with_i32(
            norm_loc,
            3,
            WebGlRenderingContext::FLOAT,
            false,
            0,
            0,
        );

        gl.clear_color(0.0, 0.0, 1.0, 1.0);
        gl.clear(
            WebGlRenderingContext::COLOR_BUFFER_BIT
                | WebGlRenderingContext::DEPTH_BUFFER_BIT,
        );

        gl.draw_elements_with_i32(
            WebGlRenderingContext::TRIANGLES,
            indices.len() as i32,
            WebGlRenderingContext::UNSIGNED_SHORT,
            0,
        );

        gl.disable_vertex_attrib_array(norm_loc);
        gl.disable_vertex_attrib_array(pos_loc);

        Ok(())
    }

    fn compile_shader(
        gl: &WebGlRenderingContext,
        shader_type: u32,
        source: &str,
    ) -> Result<WebGlShader, String> {
        let shader = gl
            .create_shader(shader_type)
            .ok_or_else(|| String::from("Unable to create shader object"))?;
        gl.shader_source(&shader, source);
        gl.compile_shader(&shader);

        if gl
            .get_shader_parameter(
                &shader,
                WebGlRenderingContext::COMPILE_STATUS,
            )
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(gl.get_shader_info_log(&shader).unwrap_or_else(|| {
                String::from("Unknown error creating shader")
            }))
        }
    }

    fn link_program(
        gl: &WebGlRenderingContext,
        vert_shader: &WebGlShader,
        frag_shader: &WebGlShader,
    ) -> Result<WebGlProgram, String> {
        let program = gl
            .create_program()
            .ok_or_else(|| String::from("Unable to create shader object"))?;

        gl.attach_shader(&program, vert_shader);
        gl.attach_shader(&program, frag_shader);
        gl.link_program(&program);

        if gl
            .get_program_parameter(&program, WebGlRenderingContext::LINK_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(program)
        } else {
            Err(gl.get_program_info_log(&program).unwrap_or_else(|| {
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

    pub fn gen_normal_matrix(&self) -> Mat4 {
        let wm = self.gen_world_matrix();
        wm.inverse().transpose()
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
    equation: Box<dyn Fn(f32, f32) -> f32>,
    gradient: Box<dyn Fn(f32, f32) -> (f32, f32)>,
}

impl Plot {
    pub fn new<F, G>(equation: F, gradient: G) -> Plot
    where
        F: Fn(f32, f32) -> f32 + 'static,
        G: Fn(f32, f32) -> (f32, f32) + 'static,
    {
        Plot {
            equation: Box::new(equation),
            gradient: Box::new(gradient),
        }
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
                vec![(x, -df_dx), (f(x, y), 1.0f32), (y, -df_dy)]
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
