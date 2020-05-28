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
struct App {}

/**
 * The view
 */
struct View<'a> {
    gl: &'a WebGlRenderingContext,
    frustum: Frustum,
    world: World,
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
struct Plot<F, G>
where
    F: Fn(f32, f32) -> f32,
    G: Fn(f32, f32) -> (f32, f32),
{
    equation: F,
    gradient: G,
}

impl<F, G> Plot<F, G>
where
    F: Fn(f32, f32) -> f32,
    G: Fn(f32, f32) -> (f32, f32),
{
    pub fn new(equation: F, gradient: G) -> Plot<F, G> {
        Plot { equation, gradient }
    }

    pub fn gen_model(&self, size: u16) -> Model {
        let fov_y = 45.0;
        let front = 0.2;
        let back = 128.0;
        let alpha = std::f32::consts::PI * 5.0 / 8.0;
        let beta = std::f32::consts::PI;
        let gamma = std::f32::consts::PI;
        let zoom = 0.8f32;
        let xtrans = 0.0f32;
        let ytrans = 0.0f32;
        let ztrans = -3.0f32;

        let unit_square = (0..size * size).map(|i| {
            let (x, y) = (i % size, i / size);
            let (x, y) =
                (x as f32 / (size - 1) as f32, y as f32 / (size - 1) as f32);
            (-1.0 + 2.0 * x, 1.0 - 2.0 * y)
        });

        let f = &self.equation;
        let del = &self.gradient;
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
