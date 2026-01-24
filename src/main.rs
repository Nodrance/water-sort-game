mod gameplay;
mod model;
mod renderer;

use crate::gameplay::*;

use macroquad::prelude::*;

#[macroquad::main("Fluid Container Simulation")]
async fn main() {
    let mut engine = GameEngine::new(true);
    loop {
        engine.render();
        if is_mouse_button_pressed(MouseButton::Left) {
            let (x, y) = mouse_position();
            engine.handle_click(x, y, false);
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            let (x, y) = mouse_position();
            engine.handle_click(x, y, true);
        }
        next_frame().await;
    }
}