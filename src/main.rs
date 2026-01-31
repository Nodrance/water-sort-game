mod gameplay;
mod model;
mod renderer;
mod solver;


use crate::gameplay::*;
use crate::solver::*;

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
        if is_key_pressed(KeyCode::S) {
            if engine.get_state().fast_is_definitely_solvable() {
                println!("The current state is solvable.");
            } else {
                println!("The current state is not solvable.");
            }
        }
        next_frame().await;
    }
}