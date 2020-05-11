use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, HtmlCanvasElement, Window, WebGlProgram, WebGlRenderingContext, WebGlShader, window};
use web_sys::*;
use web_sys::console;
use glam::Mat4;
use glam::f32::Vec3;

const WIDTH: u32 = 900;
const HEIGHT: u32 = 900;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_many(a: &str, b: &str);
}


#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = document.get_element_by_id("canvas").unwrap();
    let canvas: web_sys::HtmlCanvasElement = canvas.dyn_into::<web_sys::HtmlCanvasElement>()?;



    let gl = canvas
        .get_context("webgl")?
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()?;

    let vert_shader = compile_shader(
        &gl,
        WebGlRenderingContext::VERTEX_SHADER,
        r#"
        attribute vec3 position;
        uniform mat4 pm;
        uniform mat4 wm;
        void main() {
            gl_Position = pm * wm * vec4(position, 1.0);
        }
    "#,
    )?;
    let frag_shader = compile_shader(
        &gl,
        WebGlRenderingContext::FRAGMENT_SHADER,
        r#"
        void main() {
            gl_FragColor = vec4(1.0, 1.0, 1.0, 1.0);
        }
    "#,
    )?;
    let program = link_program(&gl, &vert_shader, &frag_shader)?;
    gl.use_program(Some(&program));

    const size: usize = 120;
    let mut v: [(f32, f32, f32, f32); size * size] = [(0.0, 0.0, 0.0, 0.0); size * size];
    for i in 0..size * size {
        let (x, y) = (i % size, i / size);
        let (x, y) = (x as f32 / (size - 1) as f32, y as f32 / (size - 1) as f32);
        v[i] = (-1.0 + 2.0 * x, 1.0 - 2.0 * y, 0.0, 0.0);
    }

    //equation
    for i in 0..size * size {
        let (x, y, w) = (v[i].0, v[i].1, v[i].3);
        v[i] = (x, y, std::f32::consts::E.powf(-(x.powi(2) + y.powi(2))), w);
    }

    let v = v;

    let mut vertices: [f32; 3 * size * size] = [0.0; 3 * size * size];
    for i in 0..size * size {
        vertices[3 * i + 0] = v[i].0;
        vertices[3 * i + 1] = v[i].1;
        vertices[3 * i + 2] = v[i].2;
    }


    let vertices = vertices;


    let mut indices = [0u16; 2 * 2 * size * size];
    {
        let mut k = 0;
        for i in 0..size {
            for j in 0..size {
                let (x, y) = (i, if i % 2 == 0 {j} else {size - j - 1});
                indices[k] = (x * size + y) as u16;
                indices[k + size] = (x * size + y) as u16;
                //indices[k] = k as u16;
                log_u32(indices[k] as u32);
                k = k + 1;
            }
        }
        for i in 0..size {
            for j in 0..size {
                let (x, y) = (i, if i % 2 == 0 {j} else {size - j - 1});
                let (x, y) = (size - x - 1, size - y - 1);
                indices[k] = (y * size + x) as u16;
                indices[k + size] = (y * size + x) as u16;
                //indices[k] = k as u16;
                log(&format!("{}: {}", k, indices[k]));
                k = k + 1;
            }
        }

    }




    let indices = indices;

    let pm = gen_projection_matrix();
    let wm = gen_world_matrix();

    draw(&gl, &program, &vertices, &indices, &pm, &wm);

    Ok(())
}

pub fn draw(gl: &web_sys::WebGlRenderingContext, program: &WebGlProgram, vertices: &[f32], indices: &[u16], pm: &Mat4, wm: &Mat4) -> Result<(), JsValue> {

    let buffer = gl.create_buffer().ok_or("failed to create buffer")?;
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

    let pm_loc = gl.get_uniform_location(program, "pm");
    gl.uniform_matrix4fv_with_f32_array(pm_loc.as_ref(), false, pm.as_ref());

    let wm_loc = gl.get_uniform_location(program, "wm");
    gl.uniform_matrix4fv_with_f32_array(wm_loc.as_ref(), false, wm.as_ref());


    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_with_i32(0, 3, WebGlRenderingContext::FLOAT, false, 0, 0);

    gl.clear_color(0.0, 0.0, 1.0, 1.0);
    gl.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

    gl.draw_elements_with_i32(
        WebGlRenderingContext::LINE_LOOP,
        indices.len() as i32,
        WebGlRenderingContext::UNSIGNED_SHORT,
        0);

    gl.disable_vertex_attrib_array(0);


    Ok(())
}

pub fn gen_projection_matrix() -> Mat4 {
    let fov_y = 45.0f32;
    let front = 0.2f32;
    let back = 128.0f32;
    let deg2rad = std::f32::consts::PI / 180.0;

    let ratio: f32 = WIDTH as f32 / HEIGHT as f32;

    Mat4::perspective_rh_gl(fov_y * deg2rad, ratio, front, back)
}

pub fn gen_world_matrix() -> Mat4 {

    let alpha = std::f32::consts::PI * 5.0 / 8.0;
    let beta = std::f32::consts::PI;
    let gamma = std::f32::consts::PI;
    let zoom = 0.8f32;
    let xtrans = 0.0f32;
    let ytrans = 0.0f32;
    let ztrans = -3.0f32;

    use glam::{Quat, Vec3};

    let rot_x = Quat::from_rotation_x(alpha);
    let rot_y = Quat::from_rotation_y(beta);
    let rot_z = Quat::from_rotation_z(gamma);

    let rot = rot_x * rot_y * rot_z;
    let trans = Vec3::new(xtrans, ytrans, ztrans);
    let scale = Vec3::new(zoom, zoom, zoom);

    Mat4::from_rotation_translation( rot, trans )


}

pub fn resize_canvas() -> Result<(), JsValue>{
    let canvas = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("canvas")
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()?;
    let window = web_sys::window().unwrap();

    let width  = window.inner_width()?.as_f64().unwrap().floor() as u32;
    let height = window.inner_height()?.as_f64().unwrap().floor() as u32;

    canvas.set_width(width);
    canvas.set_height(height);

    log("resize!");

    Ok(())
}

pub fn gen_vertices(size: u16, vertices: &mut [(f32, f32)]) -> () {
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
