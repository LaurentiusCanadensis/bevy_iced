use crate::conversions;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    message::MessageReader,
    system::{Res, ResMut, SystemParam},
};
use bevy_ecs::resource::Resource;
use bevy_input::keyboard::KeyCode;
use bevy_input::touch::TouchInput;
use bevy_input::{
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
    ButtonInput, ButtonState,
};
use bevy_window::{CursorEntered, CursorLeft, CursorMoved};
use iced_core::{keyboard, mouse, Event as IcedEvent, Point};

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedEventQueue(pub Vec<iced_core::Event>);

#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: MessageReader<'w, 's, CursorEntered>,
    cursor_left: MessageReader<'w, 's, CursorLeft>,
    cursor: MessageReader<'w, 's, CursorMoved>,
    mouse_button: MessageReader<'w, 's, MouseButtonInput>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    keyboard_input: MessageReader<'w, 's, KeyboardInput>,
    touch_input: MessageReader<'w, 's, TouchInput>,
}

fn compute_modifiers(input_map: &ButtonInput<KeyCode>) -> keyboard::Modifiers {
    let mut modifiers = keyboard::Modifiers::default();
    if input_map.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        modifiers |= keyboard::Modifiers::CTRL;
    }
    if input_map.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        modifiers |= keyboard::Modifiers::SHIFT;
    }
    if input_map.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        modifiers |= keyboard::Modifiers::ALT;
    }
    if input_map.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
        modifiers |= keyboard::Modifiers::LOGO;
    }
    modifiers
}

pub fn process_input(
    mut events: InputEvents,
    mut event_queue: ResMut<IcedEventQueue>,
    input_map: Res<ButtonInput<KeyCode>>,
) {
    event_queue.0.clear();

    for ev in events.cursor.read() {
        event_queue.0.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.read() {
        let button = conversions::mouse_button(ev.button);
        event_queue.0.push(IcedEvent::Mouse(match ev.state {
            ButtonState::Pressed => iced_core::mouse::Event::ButtonPressed(button),
            ButtonState::Released => iced_core::mouse::Event::ButtonReleased(button),
        }));
    }

    for _ev in events.cursor_entered.read() {
        event_queue.0.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorEntered));
    }

    for _ev in events.cursor_left.read() {
        event_queue.0.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorLeft));
    }

    for ev in events.mouse_wheel.read() {
        event_queue.0.push(IcedEvent::Mouse(iced_core::mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: ev.x, y: ev.y },
        }));
    }

    let modifiers = compute_modifiers(&input_map);

    for ev in events.keyboard_input.read() {
        use keyboard::Event::*;
        let event = match ev.key_code {
            KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight => ModifiersChanged(modifiers),
            _ => {
                let key = conversions::key_code(&ev.logical_key);
                if ev.state.is_pressed() {
                    KeyPressed {
                        key: key.clone(),
                        modified_key: key,
                        physical_key: keyboard::key::Physical::Unidentified(
                            iced_core::keyboard::key::NativeCode::Unidentified,
                        ),
                        modifiers,
                        location: keyboard::Location::Standard,
                        text: None,
                        repeat: false,
                    }
                } else {
                    KeyReleased {
                        key: key.clone(),
                        modified_key: key,
                        physical_key: keyboard::key::Physical::Unidentified(
                            iced_core::keyboard::key::NativeCode::Unidentified,
                        ),
                        modifiers,
                        location: keyboard::Location::Standard,
                    }
                }
            }
        };

        event_queue.0.push(IcedEvent::Keyboard(event));
    }

    for ev in events.touch_input.read() {
        event_queue.0.push(IcedEvent::Touch(conversions::touch_event(ev)));
    }
}
