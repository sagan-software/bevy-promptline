//! Console keyboard dispatch preserves event order and IME ownership.
use bevy::ecs::{message::MessageCursor, schedule::ScheduleCleanupPolicy};
use bevy::input::{
    ButtonState,
    keyboard::{Key, KeyboardFocusLost, KeyboardInput},
};
use bevy::input_focus::{
    InputDispatchPlugin, InputFocus, InputFocusSystems, dispatch_focused_input,
};
use bevy::prelude::*;
use bevy::text::{EditableText, TextEdit};
use bevy::window::PrimaryWindow;

/// Retains the typed keyboard timeline and its private Bevy event factory.
#[derive(Resource)]
pub(super) struct ChronologicalKeyboard {
    /// Independent cursor consumes each original keyboard message once.
    reader: MessageCursor<KeyboardInput>,
    /// Window focus loss ends the current held-key epoch.
    focus_lost: MessageCursor<KeyboardFocusLost>,
    /// Logical keys reflect the last processed message rather than the frame's final state.
    keys: ButtonInput<Key>,
    /// Reusable bridge lets Bevy construct its opaque focused-input event.
    bridge: Messages<KeyboardInput>,
    /// Private registered factory retains its own message cursor.
    factory: bevy::ecs::system::SystemId,
}
impl ChronologicalKeyboard {
    /// Seeds held keys and registers the factory without dispatching input.
    fn new(world: &mut World) -> Self {
        // The UI plugin initializes logical input before installing this dispatcher.
        let keys = world
            .get_resource::<ButtonInput<Key>>()
            .expect("PromptlineUiPlugin initializes logical keys")
            .clone();
        Self {
            reader: MessageCursor::default(),
            focus_lost: MessageCursor::default(),
            keys,
            bridge: Messages::default(),
            factory: world.register_system(dispatch_focused_input::<KeyboardInput>),
        }
    }
}

/// Separates console editing from the host's ordinary focused-input route.
#[derive(Clone, Copy)]
enum DispatchTarget {
    /// A live console editor receives chronological key snapshots.
    Console(Entity),
    /// Other widgets retain Bevy's batch dispatch and final key state.
    Other,
}

/// Replaces only Bevy's standard keyboard dispatcher when the host installed it.
/// Bevy 0.20.0 controls this compatibility hook:
/// <https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_input_focus/src/lib.rs>
/// and <https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ecs/src/schedule/schedule.rs>.
pub(super) fn install(app: &mut App) {
    // A host without the standard dispatcher keeps its own input integration.
    if !app.is_plugin_added::<InputDispatchPlugin>() {
        return;
    }
    let removed = app
        .world_mut()
        .try_schedule_scope(PreUpdate, |world, schedule| {
            schedule.remove_systems_in_set(
                dispatch_focused_input::<KeyboardInput>,
                world,
                ScheduleCleanupPolicy::RemoveSystemsOnly,
            )
        });
    // A replaced or unavailable system cannot be replaced a second time.
    let Ok(Ok(removed)) = removed else {
        return;
    };
    if removed == 0 {
        return;
    }
    let timeline = ChronologicalKeyboard::new(app.world_mut());
    // Keyboard dispatch precedes Bevy's remaining focused-input dispatchers.
    app.insert_resource(timeline)
        .add_message::<KeyboardFocusLost>()
        .add_systems(
            PreUpdate,
            dispatch
                .in_set(InputFocusSystems::Dispatch)
                .after(bevy::input::InputSystems)
                .before(dispatch_focused_input::<bevy::input::gamepad::GamepadButtonChangedEvent>)
                .before(dispatch_focused_input::<bevy::input::mouse::MouseWheel>),
        );
}

/// Keeps the earlier correction only for caller-owned dispatch integrations.
pub(super) const fn is_legacy_dispatch(installed: Option<Res<'_, ChronologicalKeyboard>>) -> bool {
    installed.is_none()
}

/// Borrows the original keyboard collection initialized by the UI plugin.
fn keyboard_messages(world: &World) -> &Messages<KeyboardInput> {
    world
        .get_resource()
        .expect("PromptlineUiPlugin initializes keyboard messages")
}

/// Resolves editing ownership without changing Bevy's focus fallback policy.
fn dispatch_target(world: &mut World) -> Option<DispatchTarget> {
    // Bevy dispatch requires exactly one primary window.
    let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
    let window = windows.single(world).ok()?;
    let focused = world.get_resource::<InputFocus>()?.get();
    let target = focused
        .filter(|entity| world.entities().contains(*entity))
        .unwrap_or(window);
    // The standard factory handles stale focus and non-console destinations itself.
    let is_console_role = matches!(
        world.get::<super::ConsoleRole>(target),
        Some(super::ConsoleRole::Input | super::ConsoleRole::Output)
    );
    let is_editable = world.get::<EditableText>(target).is_some();
    Some(if is_console_role && is_editable {
        DispatchTarget::Console(target)
    } else {
        DispatchTarget::Other
    })
}

/// Borrows the private timeline while callbacks retain access to the world.
fn dispatch(world: &mut World) {
    // A caller removing private integration state also disables its dispatch work.
    world
        .try_resource_scope(|world, mut timeline: Mut<'_, ChronologicalKeyboard>| {
            dispatch_frame(world, &mut timeline);
        })
        .unwrap_or(());
}

/// Processes the current frame's input.
/// Callback-generated messages wait for the next update.
fn dispatch_frame(world: &mut World, timeline: &mut ChronologicalKeyboard) {
    let Some(target) = dispatch_target(world) else {
        release_lost_focus(world, timeline);
        return;
    };
    // Held keys persist across frames; transient flags and bridge payload do not.
    timeline.keys.clear();
    timeline.bridge.clear();
    let event_count = timeline.reader.len(keyboard_messages(world));
    // Console messages stream individually; other widgets retain batch semantics.
    for _ in 0..event_count {
        let event = timeline
            .reader
            .read(keyboard_messages(world))
            .next()
            .cloned();
        let Some(event) = event else {
            break;
        };
        // Each transition precedes the focused observer that reads its modifier state.
        match event.state {
            ButtonState::Pressed => timeline.keys.press(event.logical_key.clone()),
            ButtonState::Released => timeline.keys.release(event.logical_key.clone()),
        }
        match target {
            DispatchTarget::Console(entity) => dispatch_console(world, timeline, entity, event),
            DispatchTarget::Other => {
                timeline.bridge.write(event);
            }
        }
    }
    // The ordinary factory preserves focus fallback even when no messages arrived.
    if matches!(target, DispatchTarget::Other) {
        dispatch_bridge(world, timeline);
    }
    release_lost_focus(world, timeline);
}

/// Exchanges chronological keys with frame keys during the existing observer call.
fn swap_keys(world: &mut World, keys: &mut ButtonInput<Key>) {
    let mut frame = world
        .get_resource_mut::<ButtonInput<Key>>()
        .expect("PromptlineUiPlugin initializes logical keys");
    std::mem::swap(frame.bypass_change_detection(), keys);
}

/// Dispatches one console event and places its omitted Space edit before the next event.
fn dispatch_console(
    world: &mut World,
    timeline: &mut ChronologicalKeyboard,
    target: Entity,
    event: KeyboardInput,
) {
    let space = omitted_space(world, target, &event, &timeline.keys);
    // Existing observers read chronological keys; later systems retain final frame keys.
    swap_keys(world, &mut timeline.keys);
    timeline.bridge.clear();
    timeline.bridge.write(event);
    dispatch_bridge(world, timeline);
    swap_keys(world, &mut timeline.keys);
    // Composition and selectable output cannot receive this insertion correction.
    if let Some(edit) = space
        && let Some(mut editor) = world.get_mut::<EditableText>(target)
    {
        editor.queue_edit(edit);
    }
}

/// Releases logical keys after focus loss, matching Bevy's physical-key reset.
/// <https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_input/src/keyboard.rs>.
fn release_lost_focus(world: &mut World, timeline: &mut ChronologicalKeyboard) {
    let messages = world
        .get_resource::<Messages<KeyboardFocusLost>>()
        .expect("chronological keyboard installation initializes focus-loss messages");
    let losses = timeline.focus_lost.read(messages).count();
    // A window-focus epoch cannot inherit a shortcut modifier from the old epoch.
    if losses != 0 {
        timeline.keys.release_all();
        world
            .get_resource_mut::<ButtonInput<Key>>()
            .expect("PromptlineUiPlugin initializes logical keys")
            .release_all();
    }
}

/// Uses Bevy's dispatcher to construct its opaque focused-input event.
fn dispatch_bridge(world: &mut World, timeline: &mut ChronologicalKeyboard) {
    // Preserve the original message resource while the factory sees this dispatch batch.
    std::mem::swap(
        world
            .get_resource_mut::<Messages<KeyboardInput>>()
            .expect("PromptlineUiPlugin initializes keyboard messages")
            .bypass_change_detection(),
        &mut timeline.bridge,
    );
    let mut injected = keyboard_messages(world).get_cursor_current();
    let result = world.run_system(timeline.factory);
    std::mem::swap(
        world
            .get_resource_mut::<Messages<KeyboardInput>>()
            .expect("PromptlineUiPlugin initializes keyboard messages")
            .bypass_change_detection(),
        &mut timeline.bridge,
    );
    // Callback-generated messages return to the original stream for the next update.
    for event in injected.read(&timeline.bridge) {
        world.write_message(event.clone());
    }
    result.expect("the registered Bevy keyboard dispatcher remains available");
}

/// Returns the Shift+Space edit omitted by Bevy 0.20.0's text-input observer.
/// <https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_ui_widgets/src/text_input.rs>.
fn omitted_space(
    world: &World,
    target: Entity,
    event: &KeyboardInput,
    keys: &ButtonInput<Key>,
) -> Option<TextEdit> {
    // Selectable output never acquires an insertion correction.
    if world.get::<super::ConsoleRole>(target) != Some(&super::ConsoleRole::Input) {
        return None;
    }
    // Modifier and press state determine Space ownership for this message.
    let is_space = event.logical_key == Key::Space;
    let is_pressed = event.state == ButtonState::Pressed;
    let is_shift = is_shift_only(keys);
    if !is_space || !is_pressed || !is_shift {
        return None;
    }
    // Composition owns keyboard text and cannot receive a duplicate edit.
    let editor = world.get::<EditableText>(target)?;
    if super::has_composition(editor) {
        return None;
    }
    event
        .text
        .as_ref()
        .filter(|text| !text.is_empty())
        .map(|text| TextEdit::Insert(text.clone()))
}

/// Bevy 0.20.0 `text_input.rs` handles unmodified Space but omits Shift+Space.
/// Forward only that omitted case after the focused input observers have queued
/// their edits.
pub(super) fn forward_shift_space(
    keys: Res<'_, ButtonInput<Key>>,
    focus: Res<'_, InputFocus>,
    mut events: MessageReader<'_, '_, KeyboardInput>,
    mut editors: Query<'_, '_, (&super::ConsoleRole, &mut EditableText)>,
) {
    // Only the upstream missing modifier combination needs an additional edit.
    let is_shift_only = is_shift_only(&keys);
    let editor = focus.get().and_then(|entity| editors.get_mut(entity).ok());
    // Drain every event even when focus or modifiers exclude this correction.
    let Some((role, mut editor)) = editor else {
        events.read().for_each(drop);
        return;
    };
    // Composition retains its own Space handling and never receives duplicates.
    let is_eligible =
        *role == super::ConsoleRole::Input && is_shift_only && !super::has_composition(&editor);
    for event in events.read() {
        if !is_eligible {
            continue;
        }
        let is_space = event.logical_key == Key::Space;
        let is_pressed = event.state == ButtonState::Pressed;
        if is_space
            && is_pressed
            && let Some(text) = event.text.as_ref().filter(|text| !text.is_empty())
        {
            editor.pending_edits.push(TextEdit::Insert(text.clone()));
        }
    }
}

/// Recognizes Shift without any shortcut modifier.
fn is_shift_only(keys: &ButtonInput<Key>) -> bool {
    // Read shortcut modifiers before deciding Space ownership.
    let is_shift = keys.pressed(Key::Shift);
    let is_control = keys.pressed(Key::Control);
    let is_alt = keys.pressed(Key::Alt);
    let is_super = keys.pressed(Key::Super);
    // Correction applies only when Shift is the sole active modifier.
    let is_shortcut = is_control || is_alt || is_super;
    is_shift && !is_shortcut
}
