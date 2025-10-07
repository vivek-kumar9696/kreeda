use once_cell::sync::Lazy;
use std::sync::Mutex;
use winit::dpi::PhysicalPosition;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};

pub struct MouseInput {
    scroll_x: f64,
    scroll_y: f64,
    x_pos: f64,
    y_pos: f64,
    last_y: f64,
    last_x: f64,
    mouse_button_pressed: [bool; 3],
    is_dragging: bool,
}

impl MouseInput {
    fn new() -> Self {
        Self {
            scroll_x: 0.0,
            scroll_y: 0.0,
            x_pos: 0.0,
            y_pos: 0.0,
            last_y: 0.0,
            last_x: 0.0,
            mouse_button_pressed: [false; 3],
            is_dragging: false,
        }
    }
    pub fn get_instance() -> &'static Mutex<MouseInput> {
        static INSTANCE: Lazy<Mutex<MouseInput>> = Lazy::new(|| Mutex::new(MouseInput::new()));

        &INSTANCE
    }

    pub fn handle_event(event: &WindowEvent) {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                let mut l = Self::get_instance().lock().unwrap();
                l.last_x = l.x_pos;
                l.last_y = l.y_pos;
                l.x_pos = position.x;
                l.y_pos = position.y;
                l.is_dragging = l.mouse_button_pressed.iter().any(|&b| b);
            }

            WindowEvent::MouseInput { state, button, .. } => {
                let mut l = Self::get_instance().lock().unwrap();
                let index = match button {
                    MouseButton::Left => 0,
                    MouseButton::Right => 1,
                    MouseButton::Middle => 2,
                    _ => return,
                };
                match state {
                    ElementState::Pressed => l.mouse_button_pressed[index] = true,
                    ElementState::Released => {
                        l.mouse_button_pressed[index] = false;
                        l.is_dragging = false;
                    }
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let mut l = Self::get_instance().lock().unwrap();
                match delta {
                    MouseScrollDelta::LineDelta(_, y) => l.scroll_y = f64::from(*y),
                    MouseScrollDelta::PixelDelta(PhysicalPosition { y, .. }) => {
                        l.scroll_y = f64::from(*y)
                    }
                }
            }

            _ => {}
        }
    }

    pub fn end_frame() {
        let mut listener = Self::get_instance().lock().unwrap();
        listener.scroll_x = 0.0;
        listener.scroll_y = 0.0;
        listener.last_x = listener.x_pos;
        listener.last_y = listener.y_pos;
    }

    //--Getters--//

    pub fn get_x() -> f64 {
        Self::get_instance().lock().unwrap().x_pos
    }

    pub fn get_y() -> f64 {
        Self::get_instance().lock().unwrap().y_pos
    }

    pub fn get_dx() -> f64 {
        let l = Self::get_instance().lock().unwrap();
        l.last_x - l.x_pos
    }

    pub fn get_dy() -> f64 {
        let l = Self::get_instance().lock().unwrap();
        l.last_y - l.y_pos
    }

    pub fn get_scroll_y() -> f64 {
        Self::get_instance().lock().unwrap().scroll_y
    }

    pub fn is_dragging() -> bool {
        Self::get_instance().lock().unwrap().is_dragging
    }

    pub fn mouse_button_down(button: usize) -> bool {
        let l = Self::get_instance().lock().unwrap();
        if button < l.mouse_button_pressed.len() {
            l.mouse_button_pressed[button]
        } else {
            false
        }
    }
}
