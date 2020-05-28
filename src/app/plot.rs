use glam::Mat4;
use wasm_bindgen::JsValue;
use web_sys::WebGlProgram;
use web_sys::WebGlRenderingContext;
use web_sys::WebGlShader;

pub struct Plot3D {
    vertices: Vec<f32>,
    indices: Vec<u16>,
    mesh: bool,
    fov_y: f32,
    front: f32,
    back: f32,
    pub alpha: f32,
    beta: f32,
    pub gamma: f32,
    zoom: f32,
    xtrans: f32,
    ytrans: f32,
    ztrans: f32,
}

impl Plot3D {
    pub fn new(size: u16, mesh: bool) -> Result<Plot3D, JsValue> {
        let alpha = std::f32::consts::PI * 5.0 / 8.0;
        let beta = std::f32::consts::PI;
        let gamma = std::f32::consts::PI;
        let zoom = 0.8f32;
        let xtrans = 0.0f32;
        let ytrans = 0.0f32;
        let ztrans = -3.0f32;

        let vertices: Vec<f32> = (0..size * size)
            .map(|i| {
                let (x, y) = (i % size, i / size);
                let (x, y) = (x as f32 / (size - 1) as f32, y as f32 / (size - 1) as f32);
                (-1.0 + 2.0 * x, 1.0 - 2.0 * y)
            })
            .flat_map(|(x, y)| {
                vec![
                    x,
                    y,
                    ((x.powi(2) + y.powi(2)) * (2.0 * std::f32::consts::PI)).sin()
                        / (2.0 * std::f32::consts::PI),
                ]
            })
            .collect();
        //todo this is hardcoded to e^(1x^2-y^2)
        //need to make it work for any equation
        let normals: Vec<(f32, f32, f32)> = (0..size * size)
            .map(|i| {
                let (x, y) = (i % size, i / size);
                let (x, y) = (x as f32 / (size - 1) as f32, y as f32 / (size - 1) as f32);
                (-1.0 + 2.0 * x, 1.0 - 2.0 * y)
            })
            .map(|(x, y)| {
                (
                    2.0 * x * std::f32::consts::E.powf(-(x.powi(2) + y.powi(2))),
                    2.0 * y * std::f32::consts::E.powf(-(x.powi(2) + y.powi(2))),
                    1.0,
                )
            })
            .collect();

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
                    (top_left, btm_right, top_right),
                    (top_left, btm_right, top_left),
                ]
            })
            .map(|t| {
                if mesh {
                    vec![t.0, t.1, t.1, t.2, t.2, t.0]
                } else {
                    vec![t.0, t.1, t.2]
                }
            })
            .flatten()
            .collect();

        Ok(Plot3D {
            vertices,
            indices,
            mesh,
            fov_y: 45.0,
            front: 0.2,
            back: 128.0,
            alpha,
            beta,
            gamma,
            zoom,
            xtrans,
            ytrans,
            ztrans,
        })
    }

    pub fn render(
        &self,
        gl: &WebGlRenderingContext,
        program: &WebGlProgram,
        width: u32,
        height: u32,
    ) -> Result<(), JsValue> {
        use log;
        log::info!("render");
        let vertices = &self.vertices;
        let indices = &self.indices;
        let pm = self.gen_projection_matrix(width, height);
        let wm = self.gen_world_matrix();
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
        gl.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&buffer));

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
        gl.uniform_matrix4fv_with_f32_array(pm_loc.as_ref(), false, pm.as_ref());

        let wm_loc = gl.get_uniform_location(&program, "wm");
        gl.uniform_matrix4fv_with_f32_array(wm_loc.as_ref(), false, wm.as_ref());

        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);

        gl.clear_color(0.0, 0.0, 1.0, 1.0);
        gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        gl.draw_elements_with_i32(
            if self.mesh {
                WebGlRenderingContext::LINES
            } else {
                WebGlRenderingContext::TRIANGLES
            },
            indices.len() as i32,
            WebGlRenderingContext::UNSIGNED_SHORT,
            0,
        );

        gl.disable_vertex_attrib_array(0);

        Ok(())
    }
    pub fn gen_projection_matrix(&self, width: u32, height: u32) -> Mat4 {
        let fov_y = self.fov_y;
        let front = self.front;
        let back = self.back;
        let deg2rad = std::f32::consts::PI / 180.0;

        let ratio: f32 = width as f32 / height as f32;

        Mat4::perspective_rh_gl(fov_y * deg2rad, ratio, front, back)
    }

    pub fn gen_world_matrix(&self) -> Mat4 {
        let alpha = self.alpha;
        let beta = self.beta;
        let gamma = self.gamma;
        let zoom = self.zoom;
        let xtrans = self.xtrans;
        let ytrans = self.ytrans;
        let ztrans = self.ztrans;

        use glam::{Quat, Vec3};

        let rot_x = Quat::from_rotation_x(alpha);
        let rot_y = Quat::from_rotation_y(beta);
        let rot_z = Quat::from_rotation_z(gamma);

        let rot = rot_x * rot_y * rot_z;
        let trans = Vec3::new(xtrans, ytrans, ztrans);
        let scale = Vec3::new(zoom, zoom, zoom);

        Mat4::from_rotation_translation(rot, trans)
    }

    pub fn compile_shader(
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
            .get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS)
            .as_bool()
            .unwrap_or(false)
        {
            Ok(shader)
        } else {
            Err(context
                .get_shader_info_log(&shader)
                .unwrap_or_else(|| String::from("Unknown error creating shader")))
        }
    }

    pub fn link_program(
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
            Err(context
                .get_program_info_log(&program)
                .unwrap_or_else(|| String::from("Unknown error creating program object")))
        }
    }
}
