//! Input system for handling keyboard and mouse input
//!
//! This module provides an InputSystem that translates user input into camera movements,
//! similar to the DistEngine C++ InputSystem.

use std::collections::HashSet;
use winit::event::{ElementState, MouseButton};
use winit::keyboard::KeyCode;
use winit::window::Window;
use tracing::{debug, warn};
use crate::component::Camera;

/// Configuration for InputSystem behavior
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// Camera movement speed in units per second
    pub move_speed: f32,
    /// Mouse sensitivity in degrees per pixel
    pub mouse_sensitivity: f32,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            move_speed: 10.0,
            mouse_sensitivity: 0.25,
        }
    }
}

/// InputSystem manages keyboard and mouse input state
/// and translates them into camera movements
pub struct InputSystem {
    // Keyboard state
    pressed_keys: HashSet<KeyCode>,

    // Mouse state
    last_mouse_pos: (f64, f64),
    mouse_buttons: HashSet<MouseButton>,
    mouse_delta: (f32, f32),

    // Movement configuration
    move_speed: f32,        // Units per second
    mouse_sensitivity: f32, // Degrees per pixel

    // First mouse movement flag
    first_mouse: bool,

    // Cursor lock state
    cursor_locked: bool,
}

impl InputSystem {
    /// Create a new InputSystem with default configuration
    pub fn new() -> Self {
        Self::with_config(InputConfig::default())
    }

    /// Create InputSystem with custom configuration
    pub fn with_config(config: InputConfig) -> Self {
        Self {
            pressed_keys: HashSet::new(),
            last_mouse_pos: (0.0, 0.0),
            mouse_buttons: HashSet::new(),
            mouse_delta: (0.0, 0.0),
            move_speed: config.move_speed,
            mouse_sensitivity: config.mouse_sensitivity,
            first_mouse: true,
            cursor_locked: false,
        }
    }

    /// Process keyboard input event
    /// Returns true if the event was handled
    pub fn on_keyboard_input(
        &mut self,
        keycode: KeyCode,
        state: ElementState,
    ) -> bool {
        match state {
            ElementState::Pressed => {
                self.pressed_keys.insert(keycode);
            }
            ElementState::Released => {
                self.pressed_keys.remove(&keycode);
            }
        };
        true
    }

    /// Process mouse button event
    /// Handles cursor locking when right button is pressed/released
    pub fn on_mouse_button(
        &mut self,
        window: &Window,
        button: MouseButton,
        state: ElementState,
    ) {
        match state {
            ElementState::Pressed => {
                self.mouse_buttons.insert(button);

                // Lock cursor when right mouse button is pressed
                if button == MouseButton::Right {
                    self.lock_cursor(window);
                }
            }
            ElementState::Released => {
                self.mouse_buttons.remove(&button);

                // Unlock cursor when right mouse button is released
                if button == MouseButton::Right {
                    self.unlock_cursor(window);
                }
            }
        }
    }

    /// Process mouse movement event
    pub fn on_mouse_move(&mut self, position: (f64, f64)) {
        if self.first_mouse {
            self.last_mouse_pos = position;
            self.first_mouse = false;
            return;
        }

        let dx = (position.0 - self.last_mouse_pos.0) as f32;
        let mut dy = (position.1 - self.last_mouse_pos.1) as f32;

        if crate::core::renderer_backend() == Some(crate::core::RendererBackendKind::Wgpu) {
            // wgpu平台修改 因为 y 轴方向相反，所以需要取反
            dy = -dy;
        }

        self.mouse_delta = (dx, dy);
        self.last_mouse_pos = position;
    }

    /// Update camera based on current input state
    /// Called every frame with delta time
    pub fn update_camera(&mut self, camera: &mut Camera, delta_time: f32) {
        // Handle keyboard movement (WASD)
        self.handle_keyboard_movement(camera, delta_time);

        // Handle mouse rotation (right button drag)
        self.handle_mouse_rotation(camera);

        // Reset delta for next frame
        self.mouse_delta = (0.0, 0.0);
    }

    /// Handle keyboard-based camera movement
    fn handle_keyboard_movement(&self, camera: &mut Camera, delta_time: f32) {
        let distance = self.move_speed * delta_time;

        if self.pressed_keys.contains(&KeyCode::KeyW) {
            camera.walk(-distance);
        }
        if self.pressed_keys.contains(&KeyCode::KeyS) {
            camera.walk(distance);
        }
        if self.pressed_keys.contains(&KeyCode::KeyA) {
            camera.strafe(-distance);
        }
        if self.pressed_keys.contains(&KeyCode::KeyD) {
            camera.strafe(distance);
        }
    }

    /// Handle mouse-based camera rotation
    fn handle_mouse_rotation(&mut self, camera: &mut Camera) {
        // Only rotate if right mouse button is pressed
        if !self.mouse_buttons.contains(&MouseButton::Right) {
            return;
        }

        // Skip if no movement
        if self.mouse_delta.0.abs() < 0.001 && self.mouse_delta.1.abs() < 0.001 {
            return;
        }

        // Convert pixel movement to radians
        // Match C++ version: 0.25 degrees per pixel
        let dx = -self.mouse_delta.0 * self.mouse_sensitivity * std::f32::consts::PI / 180.0;
        let dy = -self.mouse_delta.1 * self.mouse_sensitivity * std::f32::consts::PI / 180.0;

        camera.pitch(dy);
        camera.rotate_y(dx);
    }

    /// Lock and hide cursor for immersive camera control
    pub fn lock_cursor(&mut self, window: &Window) {
        if self.cursor_locked {
            return;
        }

        // Hide cursor
        window.set_cursor_visible(false);

        // Try to grab cursor (confine to window)
        // Use Confined mode as it's more widely supported than Locked
        if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::Confined) {
            // Try Locked mode as fallback
            if let Err(e2) = window.set_cursor_grab(winit::window::CursorGrabMode::Locked) {
                warn!(
                    "Failed to grab cursor (Confined: {}, Locked: {}). Cursor will remain visible but rotation still works.",
                    e, e2
                );
            } else {
                debug!("Cursor grabbed with Locked mode");
                self.cursor_locked = true;
            }
        } else {
            debug!("Cursor grabbed with Confined mode");
            self.cursor_locked = true;
        }
    }

    /// Unlock and show cursor
    pub fn unlock_cursor(&mut self, window: &Window) {
        if !self.cursor_locked {
            // Always restore cursor visibility even if not locked
            window.set_cursor_visible(true);
            return;
        }

        // Show cursor
        window.set_cursor_visible(true);

        // Release cursor grab
        if let Err(e) = window.set_cursor_grab(winit::window::CursorGrabMode::None) {
            warn!("Failed to release cursor grab: {}", e);
        } else {
            debug!("Cursor grab released");
        }

        self.cursor_locked = false;
    }

    /// Reset mouse state (useful when window loses focus)
    pub fn reset_mouse(&mut self) {
        self.mouse_delta = (0.0, 0.0);
        self.first_mouse = true;
    }

    /// Check if a specific key is currently pressed
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    /// Check if a specific mouse button is currently pressed
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.mouse_buttons.contains(&button)
    }

    /// Get the current movement speed
    pub fn move_speed(&self) -> f32 {
        self.move_speed
    }

    /// Set the movement speed
    pub fn set_move_speed(&mut self, speed: f32) {
        self.move_speed = speed;
    }

    /// Get the current mouse sensitivity
    pub fn mouse_sensitivity(&self) -> f32 {
        self.mouse_sensitivity
    }

    /// Set the mouse sensitivity
    pub fn set_mouse_sensitivity(&mut self, sensitivity: f32) {
        self.mouse_sensitivity = sensitivity;
    }
}

impl Default for InputSystem {
    fn default() -> Self {
        Self::new()
    }
}
