use log;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
use web_sys::WebGlRenderingContext;

pub mod plot;
use plot::Plot3D;

#[wasm_bindgen]
pub struct App {
    canvas: HtmlCanvasElement,
    plot: Rc<RefCell<Plot3D>>,
    mouse_pos: Rc<Cell<(i32, i32)>>,
    last_mouse_pos: Rc<Cell<(i32, i32)>>,
    mouse_down: Rc<Cell<bool>>,
}

impl App {
    pub fn new(canvas: HtmlCanvasElement, plot: Plot3D) -> Result<App, JsValue> {
        let canvas = canvas;
        let mouse_pos = Rc::new(Cell::new((0, 0)));
        let last_mouse_pos = Rc::new(Cell::new((0, 0)));
        let mouse_down = Rc::new(Cell::new(false));
        let plot = Rc::new(RefCell::new(plot));

        let app = App {
            canvas,
            plot,
            mouse_pos,
            last_mouse_pos,
            mouse_down,
        };

        app.add_mouse_listener()?;
        Ok(app)
    }

    pub fn update(&mut self) {
        let plot = &mut self.plot.borrow_mut();
        let (x, y) = self.mouse_pos.get();
        let (last_x, last_y) = self.mouse_pos.get();
        plot.gamma = plot.gamma - (last_x as f32 - x as f32).atan();
        plot.alpha = plot.alpha - (last_y as f32 - y as f32).atan();
        log::debug!("{:?}", self.mouse_pos.get());
    }

    pub fn render(&self) -> Result<(), JsValue> {
        let plot = &mut self.plot.borrow_mut();

        let gl = self
            .canvas
            .get_context("webgl")?
            .unwrap()
            .dyn_into::<WebGlRenderingContext>()?;

        let vert_shader = Plot3D::compile_shader(
            &gl,
            WebGlRenderingContext::VERTEX_SHADER,
            include_str!("shaders/vertex.glsl"),
        )?;
        let frag_shader = Plot3D::compile_shader(
            &gl,
            WebGlRenderingContext::FRAGMENT_SHADER,
            include_str!("shaders/fragment.glsl"),
        )?;

        let program = Plot3D::link_program(&gl, &vert_shader, &frag_shader)?;
        Ok(plot.render(&gl, &program, self.canvas.width(), self.canvas.height())?)
    }

    fn add_mouse_listener(&self) -> Result<(), JsValue> {
        let mouse_pos = self.mouse_pos.clone();
        let mouse_down = self.mouse_down.clone();
        let last_mouse_pos = self.last_mouse_pos.clone();
        self.add_event_listener_with_callback("mousemove", move |event: web_sys::MouseEvent| {
            mouse_pos.set((event.client_x(), event.client_y()));
            if mouse_down.get() {
                log::info!("{:?}", mouse_pos.get());
            } else {
                last_mouse_pos.set(mouse_pos.get());
            }
        })?;

        let mouse_down = self.mouse_down.clone();
        let last_mouse_pos = self.last_mouse_pos.clone();
        self.add_event_listener_with_callback("mousedown", move |event: web_sys::MouseEvent| {
            log::info!("mousedown");
            last_mouse_pos.set((event.client_x(), event.client_y()));
            mouse_down.set(true);
        })?;

        let mouse_down = self.mouse_down.clone();
        let listener = move |_: web_sys::MouseEvent| {
            log::info!("mouseup");
            mouse_down.set(false);
        };
        self.add_event_listener_with_callback("mouseup", listener)?;
        Ok(())
    }

    fn add_event_listener_with_callback<F>(&self, type_: &str, listener: F) -> Result<(), JsValue>
    where
        F: 'static + FnMut(web_sys::MouseEvent),
    {
        let closure = Closure::wrap(Box::new(listener) as Box<dyn FnMut(_)>);
        self.canvas
            .add_event_listener_with_callback(type_, closure.as_ref().unchecked_ref())?;
        closure.forget();
        Ok(())
    }
}
