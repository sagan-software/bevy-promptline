//! Default UI construction without a window or renderer.
use bevy::prelude::*;
use bevy_promptline::input::{ConsoleRole, ConsoleState, PromptlineUiPlugin};

#[test]
fn long_output_has_an_ancestor_clip_and_input_does_not_shrink() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let mut roles = app.world_mut().query::<(Entity, &ConsoleRole, &Node)>();
    let output = roles
        .iter(app.world())
        .find_map(|(entity, role, _)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let input = roles
        .iter(app.world())
        .find_map(|(_, role, node)| (*role == ConsoleRole::Input).then_some(node))
        .unwrap();
    assert_eq!(input.flex_shrink.to_bits(), 0.0_f32.to_bits());
    let parent = app.world().get::<ChildOf>(output).unwrap().parent();
    let viewport = app.world().get::<Node>(parent).unwrap();
    assert_eq!(viewport.overflow, Overflow::scroll());
    assert_eq!(viewport.min_height, px(32));
}

#[test]
fn default_ui_exposes_replaceable_roles_without_creating_a_camera_or_window() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let roles: Vec<_> = app
        .world_mut()
        .query::<&ConsoleRole>()
        .iter(app.world())
        .copied()
        .collect();
    for role in [
        ConsoleRole::Root,
        ConsoleRole::Input,
        ConsoleRole::Output,
        ConsoleRole::Help,
        ConsoleRole::Ghost,
        ConsoleRole::Candidates,
        ConsoleRole::OpenControl,
    ] {
        assert!(roles.contains(&role));
    }
    assert!(!app.world().resource::<ConsoleState>().is_editing());
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut().query::<&Window>().iter(app.world()).count(),
        0
    );
}

#[test]
fn opening_acquires_editor_focus_and_closing_restores_a_valid_prior_target() {
    use bevy::input_focus::{FocusCause, InputFocus};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let previous = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(previous, FocusCause::Navigated);
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(input));
    assert!(app.world().resource::<ConsoleState>().is_editing());
    app.world_mut().resource_mut::<ConsoleState>().close();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(previous));
}

#[test]
fn repeated_open_and_close_requests_preserve_focus_ownership() {
    use bevy::input_focus::{FocusCause, InputFocus};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let previous = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(previous, FocusCause::Navigated);
    app.world_mut().resource_mut::<ConsoleState>().close();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(previous));
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().close();
    app.update();
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(previous));
}

/// Typed fixture used by the UI submission boundary.
#[derive(Debug)]
struct Echo(String);
impl bevy_promptline::ConsoleCommand for Echo {
    fn metadata() -> Result<bevy_promptline::CommandMetadata, bevy_promptline::InvalidCommandName> {
        bevy_promptline::CommandMetadata::new("echo", "Print text")
    }
    fn parse(arguments: &[String]) -> Result<Self, String> {
        Ok(Self(arguments.join(" ")))
    }
}

#[test]
fn enter_delivers_the_actual_editor_value_once_and_clears_only_accepted_input() {
    use bevy::text::EditableText;
    use bevy_promptline::{CommandOutcome, Output, PromptlineAppExt, PromptlineRuntimePlugin};
    let mut app = App::new();
    app.add_plugins((PromptlineRuntimePlugin, PromptlineUiPlugin));
    app.register_command::<Echo, _>(|In(echo): In<Echo>, mut output: ResMut<'_, Output>| {
        output.write(echo.0);
        CommandOutcome::Success
    })
    .unwrap();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("echo actual");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["actual"]
    );
    assert_eq!(
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .to_string(),
        ""
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("unknown");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Enter);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .to_string(),
        "unknown"
    );
}

#[test]
fn tab_accepts_a_command_preview_without_submitting_it() {
    use bevy::text::EditableText;
    use bevy_promptline::{CommandOutcome, Output, PromptlineAppExt};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| -> CommandOutcome {
        panic!("completion dispatched a command")
    })
    .unwrap();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("ec");
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Tab);
    app.update();
    assert_eq!(
        app.world()
            .get::<EditableText>(input)
            .unwrap()
            .value()
            .to_string(),
        "echo"
    );
    assert_eq!(app.world().resource::<Output>().texts().count(), 0);
}

#[test]
fn history_keys_recall_commands_restore_drafts_and_restart_after_manual_edits() {
    use bevy::text::EditableText;
    use bevy_promptline::command::{CommandHistory, Sensitivity};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    app.world_mut()
        .resource_mut::<CommandHistory>()
        .record("echo old", Sensitivity::Public);
    app.world_mut()
        .resource_mut::<CommandHistory>()
        .record("echo new", Sensitivity::Public);
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("draft");
    for (key, expected) in [
        (KeyCode::ArrowUp, "echo new"),
        (KeyCode::ArrowUp, "echo old"),
        (KeyCode::ArrowDown, "echo new"),
        (KeyCode::ArrowDown, "draft"),
    ] {
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        assert_eq!(
            app.world()
                .get::<EditableText>(input)
                .unwrap()
                .value()
                .to_string(),
            expected
        );
    }
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("changed draft");
    for (key, expected) in [
        (KeyCode::ArrowUp, "echo new"),
        (KeyCode::ArrowDown, "changed draft"),
    ] {
        *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        assert_eq!(
            app.world()
                .get::<EditableText>(input)
                .unwrap()
                .value()
                .to_string(),
            expected
        );
    }
}

#[test]
fn invalid_drafts_clear_stale_discovery_after_policy_changes() {
    use bevy::text::EditableText;
    use bevy_promptline::{CommandOutcome, CommandRegistry, PromptlineAppExt};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| CommandOutcome::Success)
        .unwrap();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("ec");
    app.update();
    let candidates = app
        .world_mut()
        .query::<(&ConsoleRole, &Text)>()
        .iter(app.world())
        .find_map(|(role, text)| (*role == ConsoleRole::Candidates).then_some(text.0.clone()))
        .unwrap();
    assert_eq!(candidates, "echo");
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(|_, _, _| false);
    app.world_mut()
        .get_mut::<EditableText>(input)
        .unwrap()
        .editor
        .set_text("\0");
    app.update();
    for (role, text) in app
        .world_mut()
        .query::<(&ConsoleRole, &Text)>()
        .iter(app.world())
    {
        if matches!(role, ConsoleRole::Candidates | ConsoleRole::Ghost) {
            let displayed = &text.0;
            assert!(displayed.is_empty(), "stale {role:?}: {displayed}");
        } else if *role == ConsoleRole::Help {
            assert_eq!(text.0, "commands cannot contain NUL");
        }
    }
}

#[test]
fn visible_editor_reacquires_ownership_after_pointer_focus_returns() {
    use bevy::input_focus::{FocusCause, InputFocus};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app.world().resource::<InputFocus>().get().unwrap();
    app.world_mut().resource_mut::<InputFocus>().clear();
    app.update();
    assert!(!app.world().resource::<ConsoleState>().is_editing());
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(input, FocusCause::Navigated);
    app.update();
    assert!(app.world().resource::<ConsoleState>().is_editing());
}

#[test]
fn closing_key_is_owned_for_its_frame_and_game_input_resumes_next_frame() {
    use bevy_promptline::input::gameplay_input_allowed;
    #[derive(Default, Resource)]
    struct GameFrames(usize);
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin)
        .init_resource::<GameFrames>();
    app.add_systems(
        Update,
        (|mut frames: ResMut<'_, GameFrames>| frames.0 += 1)
            .run_if(gameplay_input_allowed)
            .after(bevy_promptline::input::PromptlineSystems::Input),
    );
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let before = app.world().resource::<GameFrames>().0;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Escape);
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().0, before);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();
    app.update();
    assert_eq!(app.world().resource::<GameFrames>().0, before + 1);
}

#[test]
fn configured_toggle_and_close_keys_replace_the_defaults() {
    use bevy_promptline::input::{ConsoleBindings, ConsoleMode};
    let mut app = App::new();
    app.insert_resource(ConsoleBindings {
        toggle: Some(KeyCode::F1),
        close: KeyCode::F2,
        ..Default::default()
    });
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::Backquote);
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::Closed
    );
    *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F1);
    app.update();
    assert!(app.world().resource::<ConsoleState>().is_editing());
    *app.world_mut().resource_mut::<ButtonInput<KeyCode>>() = ButtonInput::default();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F2);
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::Closed
    );
}

#[test]
fn theme_changes_update_public_roles_and_the_authoritative_editor_cursor() {
    use bevy::text::TextCursorStyle;
    use bevy_promptline::input::ConsoleTheme;
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let background = Color::WHITE;
    let foreground = Color::BLACK;
    app.insert_resource(ConsoleTheme {
        background,
        input: foreground,
        ghost: Color::srgb(0.2, 0.3, 0.2),
        font: TextFont {
            font_size: FontSize::Px(24.0),
            ..Default::default()
        },
        ..Default::default()
    });
    app.update();
    for (role, color) in app
        .world_mut()
        .query::<(&ConsoleRole, &BackgroundColor)>()
        .iter(app.world())
    {
        if *role == ConsoleRole::Root {
            assert_eq!(color.0, background);
        }
    }
    let (_, text_color, font, cursor) = app
        .world_mut()
        .query::<(&ConsoleRole, &TextColor, &TextFont, &TextCursorStyle)>()
        .iter(app.world())
        .find(|(role, ..)| **role == ConsoleRole::Input)
        .unwrap();
    assert_eq!(text_color.0, foreground);
    assert!(matches!(font.font_size, FontSize::Px(size) if (size - 24.0).abs() < f32::EPSILON));
    assert_eq!(cursor.color, app.world().resource::<ConsoleTheme>().caret);
}

#[test]
fn showing_and_releasing_preserve_visible_console_without_editing_ownership() {
    use bevy_promptline::input::ConsoleMode;
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().show();
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::OpenReleased
    );
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    assert!(app.world().resource::<ConsoleState>().is_editing());
    app.world_mut()
        .resource_mut::<ConsoleState>()
        .release_focus();
    app.update();
    assert_eq!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::OpenReleased
    );
}

#[test]
fn shift_space_queues_the_space_ignored_by_bevy_text_input() {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    use bevy::input_focus::InputFocus;
    use bevy::text::{EditableText, TextEdit};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin)
        .add_message::<KeyboardInput>()
        .init_resource::<ButtonInput<Key>>();
    app.update();
    app.world_mut().resource_mut::<ConsoleState>().open();
    app.update();
    let input = app.world().resource::<InputFocus>().get().unwrap();
    app.world_mut()
        .resource_mut::<ButtonInput<Key>>()
        .press(Key::Shift);
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Space,
        logical_key: Key::Space,
        state: ButtonState::Pressed,
        text: Some(" ".into()),
        repeat: false,
        window,
    });
    app.update();
    let spaces = app
        .world()
        .get::<EditableText>(input)
        .unwrap()
        .pending_edits
        .iter()
        .filter(|edit| matches!(edit, TextEdit::Insert(value) if value == " "))
        .count();
    assert_eq!(spaces, 1);
}

/// Local editing guidance remains readable without submitting or clearing the draft.
#[test]
fn incomplete_syntax_displays_guidance_without_changing_the_editor() {
    use bevy::text::EditableText;
    use bevy_promptline::{CommandNotification, PromptlineAppExt};
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.register_command::<Echo, _>(|_: In<Echo>| bevy_promptline::CommandOutcome::Success)
        .unwrap();
    app.update();
    let input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    for (draft, expected) in [
        ("echo 'unfinished", "Finish the quoted argument"),
        ("echo tail\\", "Enter the character after the backslash"),
        ("unknown", "Unknown command"),
        ("echo\0", "commands cannot contain NUL"),
        ("echo\n", "commands must contain one line"),
    ] {
        app.world_mut()
            .get_mut::<EditableText>(input)
            .unwrap()
            .editor
            .set_text(draft);
        app.update();
        let help = app
            .world_mut()
            .query::<(&ConsoleRole, &Text)>()
            .iter(app.world())
            .find_map(|(role, text)| (*role == ConsoleRole::Help).then_some(text.0.clone()))
            .unwrap();
        assert_eq!(help, expected);
        assert_eq!(
            app.world()
                .get::<EditableText>(input)
                .unwrap()
                .editor
                .raw_text(),
            draft
        );
        assert!(
            app.world()
                .resource::<Messages<CommandNotification>>()
                .is_empty()
        );
    }
}

/// Console editors receive Bevy 0.20's `TextInput` marker in either insertion order.
#[test]
fn console_editors_receive_text_input_and_other_editors_do_not() {
    use bevy::text::EditableText;
    use bevy::ui_widgets::TextInput;

    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let default_input = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity))
        .unwrap();
    let role_first = app.world_mut().spawn(ConsoleRole::Output).id();
    app.world_mut()
        .entity_mut(role_first)
        .insert(EditableText::default());
    let editor_first = app.world_mut().spawn(EditableText::default()).id();
    app.world_mut()
        .entity_mut(editor_first)
        .insert(ConsoleRole::Input);
    // Reinserting a role keeps the existing marker.
    app.world_mut()
        .entity_mut(editor_first)
        .insert(ConsoleRole::Input);
    let plain_role = app.world_mut().spawn(ConsoleRole::Help).id();
    let other = app.world_mut().spawn(EditableText::default()).id();
    app.update();
    for entity in [default_input, role_first, editor_first] {
        assert!(app.world().get::<TextInput>(entity).is_some());
    }
    assert!(app.world().get::<TextInput>(plain_role).is_none());
    assert!(app.world().get::<TextInput>(other).is_none());
}

/// Clicking the visible open control opens the console through Bevy 0.20's flat pointer event.
#[test]
fn open_control_click_opens_the_console() {
    use bevy::picking::backend::HitData;
    use bevy::picking::pointer::{Location, PointerId};
    use bevy_promptline::input::ConsoleMode;

    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.update();
    let control = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::OpenControl).then_some(entity))
        .unwrap();
    let location = Location {
        target: bevy::camera::RenderTarget::None {
            size: UVec2::new(200, 100),
        }
        .normalize(None)
        .unwrap(),
        position: Vec2::ZERO,
    };
    app.world_mut().trigger(PointerClick {
        entity: control,
        pointer: Pointer::new(PointerId::Mouse, location),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0.0, None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    assert_ne!(
        app.world().resource::<ConsoleState>().mode(),
        ConsoleMode::Closed
    );
}
