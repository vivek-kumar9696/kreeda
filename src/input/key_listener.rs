use once_cell::sync::Lazy;
use std::sync::Mutex;
use std::collections::HashSet;
use winit::keyboard::Key;
use winit::event::{ElementState, WindowEvent};

pub struct KeyInput {
    keys_pressed: HashSet<Key>,
    keys_just_pressed: HashSet<Key>,
    keys_just_released: HashSet<Key>,
}

impl KeyInput {
    fn new() -> Self {
        Self {
            keys_pressed: HashSet::new(),
            keys_just_pressed: HashSet::new(),
            keys_just_released: HashSet::new(),
        }
    }
    pub fn get_instance() -> &'static Mutex<KeyInput> {
        static INSTANCE: Lazy<Mutex<KeyInput>> = Lazy::new(|| Mutex::new(KeyInput::new()));

        &INSTANCE
    }

    pub fn handle_event(event: &WindowEvent) {
        if let WindowEvent::KeyboardInput { event, .. } = event {
            let mut input = Self::get_instance().lock().unwrap();

            match event.state {
                ElementState::Pressed => {
                    if !input.keys_pressed.contains(&event.logical_key) {
                        input.keys_just_pressed.insert(event.logical_key.clone());
                    }
                    input.keys_pressed.insert(event.logical_key.clone());
                }
                ElementState::Released => {
                    input.keys_pressed.remove(&event.logical_key);
                    input.keys_just_released.insert(event.logical_key.clone());
                }
            }
        }
    }

    pub fn end_frame() {
        let mut input = Self::get_instance().lock().unwrap();
        input.keys_just_pressed.clear();
        input.keys_just_released.clear();
    }

    //--Getters--//

    pub fn key_down(key: &Key) -> bool {
        Self::get_instance().lock().unwrap().keys_pressed.contains(key)
    }

    pub fn key_just_pressed(key: &Key) -> bool {
        Self::get_instance().lock().unwrap().keys_just_pressed.contains(key)
    }

    pub fn key_just_released(key: &Key) -> bool {
        Self::get_instance().lock().unwrap().keys_just_released.contains(key)
    }
}

