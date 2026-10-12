//! Focused keyboard events preserve modifier transitions and edit order within a frame.
use bevy::input::{
    ButtonState, InputPlugin,
    keyboard::{Key, KeyboardInput},
};
use bevy::input_focus::{InputDispatchPlugin, InputFocus, InputFocusPlugin};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::ui_widgets::{TextInput, TextInputPlugin};
use bevy::window::PrimaryWindow;
use bevy_promptline::input::{ConsoleState, PromptlineUiPlugin};

/// Installs the actual Bevy keyboard dispatcher and editable-text observers without a renderer.
fn keyboard_app() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins((
        InputPlugin,
        InputFocusPlugin,
        InputDispatchPlugin,
        TextInputPlugin,
        PromptlineUiPlugin,
    ))
    .init_resource::<UiScale>()
    // Bevy 0.20 text input autoscroll reads real time and pointer state.
    .init_resource::<Time<Real>>()
    .init_resource::<PointerState>()
    .add_message::<Ime>()
    .add_message::<PointerRelease>();
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.finish();
    app.cleanup();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app.world().resource::<InputFocus>().get().unwrap();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .pending_edits
        .clear();
    (app, input, window)
}

/// Builds one typed platform event with its original text and modifier transition.
fn key(
    window: Entity,
    code: KeyCode,
    logical: Key,
    state: ButtonState,
    text: Option<&str>,
) -> KeyboardInput {
    KeyboardInput {
        key_code: code,
        logical_key: logical,
        state,
        text: text.map(Into::into),
        repeat: false,
        window,
    }
}

/// Reads the observable edit queue without requiring font shaping or GPU resources.
fn edits(app: &App, input: Entity) -> Vec<String> {
    app.world()
        .get::<EditableText>(input)
        .unwrap()
        .pending_edits
        .iter()
        .map(|edit| match edit {
            TextEdit::Insert(text) => format!("insert:{text}"),
            TextEdit::SelectAll => "select-all".into(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn space_precedes_a_shifted_quote_in_the_same_frame() {
    let (mut app, input, window) = keyboard_app();
    for event in [
        key(
            window,
            KeyCode::KeyA,
            Key::Character("a".into()),
            ButtonState::Pressed,
            Some("a"),
        ),
        key(
            window,
            KeyCode::Space,
            Key::Space,
            ButtonState::Pressed,
            Some(" "),
        ),
        key(
            window,
            KeyCode::ShiftLeft,
            Key::Shift,
            ButtonState::Pressed,
            None,
        ),
        key(
            window,
            KeyCode::Quote,
            Key::Character("\"".into()),
            ButtonState::Pressed,
            Some("\""),
        ),
    ] {
        app.world_mut().write_message(event);
    }
    app.update();
    assert_eq!(edits(&app, input), ["insert:a", "insert: ", "insert:\""]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn control_select_all_keeps_plain_text_on_both_sides_of_the_shortcut() {
    let (mut app, input, window) = keyboard_app();
    for event in [
        key(
            window,
            KeyCode::KeyA,
            Key::Character("a".into()),
            ButtonState::Pressed,
            Some("a"),
        ),
        key(
            window,
            KeyCode::KeyA,
            Key::Character("a".into()),
            ButtonState::Released,
            None,
        ),
        key(
            window,
            KeyCode::ControlLeft,
            Key::Control,
            ButtonState::Pressed,
            None,
        ),
        key(
            window,
            KeyCode::KeyA,
            Key::Character("a".into()),
            ButtonState::Pressed,
            Some("a"),
        ),
        key(
            window,
            KeyCode::ControlLeft,
            Key::Control,
            ButtonState::Released,
            None,
        ),
        key(
            window,
            KeyCode::KeyB,
            Key::Character("b".into()),
            ButtonState::Pressed,
            Some("b"),
        ),
    ] {
        app.world_mut().write_message(event);
    }
    app.update();
    assert_eq!(edits(&app, input), ["insert:a", "select-all", "insert:b"]);
}

#[test]
fn held_shift_keeps_space_and_quote_in_order_without_changing_frame_keys() {
    let (mut app, input, window) = keyboard_app();
    app.world_mut().write_message(key(
        window,
        KeyCode::ShiftLeft,
        Key::Shift,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    for event in [
        key(
            window,
            KeyCode::Space,
            Key::Space,
            ButtonState::Pressed,
            Some(" "),
        ),
        key(
            window,
            KeyCode::Quote,
            Key::Character("\"".into()),
            ButtonState::Pressed,
            Some("\""),
        ),
    ] {
        app.world_mut().write_message(event);
    }
    app.update();
    assert_eq!(edits(&app, input), ["insert: ", "insert:\""]);
    assert!(
        app.world()
            .resource::<ButtonInput<Key>>()
            .pressed(Key::Shift)
    );
    assert_eq!(app.world().resource::<Messages<KeyboardInput>>().len(), 2);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn another_widget_keeps_bevys_existing_batch_keyboard_semantics() {
    use bevy::input_focus::FocusCause;
    let (mut app, _, window) = keyboard_app();
    app.world_mut().resource_mut::<ConsoleState>().close();
    app.update();
    // Bevy 0.20 routes widget input only to editors that carry `TextInput`.
    let other = app
        .world_mut()
        .spawn((EditableText::new("other"), TextInput))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(other, FocusCause::Navigated);
    for event in [
        key(
            window,
            KeyCode::ControlLeft,
            Key::Control,
            ButtonState::Pressed,
            None,
        ),
        key(
            window,
            KeyCode::KeyA,
            Key::Character("a".into()),
            ButtonState::Pressed,
            Some("a"),
        ),
        key(
            window,
            KeyCode::ControlLeft,
            Key::Control,
            ButtonState::Released,
            None,
        ),
    ] {
        app.world_mut().write_message(event);
    }
    app.update();
    assert_eq!(edits(&app, other), ["TextEnd(false)", "insert:a"]);
    assert!(
        !app.world()
            .resource::<ButtonInput<Key>>()
            .pressed(Key::Control)
    );
}

#[test]
fn active_composition_owns_shift_space_without_an_extra_edit() {
    use bevy::text::{FontCx, LayoutCx};
    let (mut app, input, window) = keyboard_app();
    let mut fonts = FontCx::default();
    let mut layouts = LayoutCx::default();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .driver(&mut fonts, &mut layouts)
        .set_compose("compose", None);
    for event in [
        key(
            window,
            KeyCode::ShiftLeft,
            Key::Shift,
            ButtonState::Pressed,
            None,
        ),
        key(
            window,
            KeyCode::Space,
            Key::Space,
            ButtonState::Pressed,
            Some(" "),
        ),
    ] {
        app.world_mut().write_message(event);
    }
    app.update();
    assert!(edits(&app, input).is_empty());
    assert!(
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .is_composing()
    );
}

#[test]
fn focused_callbacks_preserve_new_keyboard_messages_for_the_next_frame() {
    use bevy::input_focus::FocusedInput;
    /// Injects a second typed event through the caller's ordinary message API.
    fn inject(event: On<'_, '_, FocusedInput<KeyboardInput>>, mut commands: Commands<'_, '_>) {
        if event.input.logical_key == Key::Character("a".into())
            && event.input.state == ButtonState::Pressed
        {
            commands.write_message(key(
                event.input.window,
                KeyCode::KeyB,
                Key::Character("b".into()),
                ButtonState::Pressed,
                Some("b"),
            ));
        }
    }
    let (mut app, input, window) = keyboard_app();
    app.add_observer(inject);
    app.world_mut().write_message(key(
        window,
        KeyCode::KeyA,
        Key::Character("a".into()),
        ButtonState::Pressed,
        Some("a"),
    ));
    app.update();
    assert_eq!(edits(&app, input), ["insert:a"]);
    assert_eq!(app.world().resource::<Messages<KeyboardInput>>().len(), 2);
    app.update();
    assert_eq!(edits(&app, input), ["insert:a", "insert:b"]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn window_focus_loss_releases_shortcut_modifiers_before_new_text() {
    use bevy::input::keyboard::KeyboardFocusLost;
    let (mut app, input, window) = keyboard_app();
    app.world_mut().write_message(key(
        window,
        KeyCode::ControlLeft,
        Key::Control,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    app.world_mut().write_message(KeyboardFocusLost);
    app.update();
    app.world_mut().write_message(key(
        window,
        KeyCode::KeyA,
        Key::Character("a".into()),
        ButtonState::Pressed,
        Some("a"),
    ));
    app.update();
    assert_eq!(edits(&app, input), ["insert:a"]);
    assert!(
        !app.world()
            .resource::<ButtonInput<Key>>()
            .pressed(Key::Control)
    );
}

/// Bevy 0.20 text input blurs on Escape; the close binding must still close the console.
#[test]
fn escape_closes_the_console_after_bevy_releases_editor_focus() {
    use bevy_promptline::input::ConsoleMode;
    let (mut app, input, window) = keyboard_app();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(input));
    app.world_mut().write_message(key(
        window,
        KeyCode::Escape,
        Key::Escape,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::Closed
    );
    assert_eq!(app.world().resource::<InputFocus>().get(), None);
}

/// A rebound close key leaves Bevy's Escape blur as a release of editing ownership.
#[test]
fn escape_releases_editing_when_another_key_closes_the_console() {
    use bevy_promptline::input::{ConsoleBindings, ConsoleMode};
    let (mut app, _, window) = keyboard_app();
    app.world_mut().resource_mut::<ConsoleBindings>().close = KeyCode::F1;
    app.world_mut().write_message(key(
        window,
        KeyCode::Escape,
        Key::Escape,
        ButtonState::Pressed,
        None,
    ));
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::OpenReleased
    );
}
