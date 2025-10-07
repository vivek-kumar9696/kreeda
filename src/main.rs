mod engine;
mod input;
use crate::engine::window::Window;

fn main() {
    let mut window = Window::get();
    window.run();
}
