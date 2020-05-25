use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;
use web_sys::WebGlRenderingContext;

pub mod plot;
use plot::Plot3D;

pub struct App {
    canvas: HtmlCanvasElement,
    plot: Plot3D,
}

impl App {
    pub fn new(canvas: HtmlCanvasElement, plot: Plot3D) -> Result<App, JsValue> {
        let canvas = canvas;

        Ok(App { canvas, plot })
    }

    pub fn render(&self) -> Result<(), JsValue> {
        Ok(self
            .plot
            .render(self.canvas.width(), self.canvas.height())?)
    }
}
