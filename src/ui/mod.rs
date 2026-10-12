//! Bevy UI roles and editor integration without owning a camera or window.
use crate::sets::PromptlineSystems;
mod bindings;
#[cfg(target_arch = "wasm32")]
mod browser;
mod colors;
mod highlight;
mod keyboard;
mod scrollback;
mod theme;
pub use self::bindings::ConsoleBindings;
pub use self::colors::SeverityColors;
pub use self::highlight::HighlightColors;
pub use self::theme::ConsoleTheme;
use bevy::input_focus::{FocusCause, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextSystems, FontCx, LayoutCx, TextCursorStyle, TextEdit};
use bevy::ui_widgets::TextInput;

/// Installs a default console tree and its input integration in an existing
/// app. Applications provide their own rendering, input, camera, and window
/// plugins.
#[derive(Debug, Default, Clone, Copy)]
pub struct PromptlineUiPlugin;

/// Identifies the purpose of an entity in a replaceable console presentation.
/// Applications can query these components without depending on private entity
/// handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component)]
pub enum ConsoleRole {
    /// The root container groups the default console presentation. Applications
    /// may move it, resize it, or assign an offscreen UI camera through
    /// ordinary components.
    Root,
    /// The authoritative single-line editable input receives keyboard and
    /// pointer
    /// operations. Applications provide an `EditableText` component when
    /// replacing this role's default entity; the UI plugin adds Bevy's
    /// `TextInput` widget marker to every console role with an editor.
    Input,
    /// The output region presents retained typed game writes and runtime
    /// diagnostics. Its role remains distinct from editable input so output
    /// cannot become a submitted command.
    Output,
    /// The contextual help region describes permitted commands while the player
    /// types. Presentations reserve its space so changing help does not move
    /// the input region.
    Help,
    /// The ghost region presents a non-authoritative completion suffix beside
    /// the caret. It remains separate from the editable value and must not
    /// enter copying or submission.
    Ghost,
    /// The candidate region presents permitted replacement choices near the
    /// input. Selection identifies a preview, while explicit completion updates
    /// the authoritative editable value.
    Candidates,
    /// The visible open control allows access without relying on the toggle
    /// key. Applications can replace its placement and styling while preserving
    /// the control's action.
    OpenControl,
    /// The drag region identifies a host-controlled window movement surface.
    /// Example systems handle its pointer capture and positioning without
    /// changing command parsing or input ownership.
    DragRegion,
    /// The close control provides visible pointer access to the console's close
    /// action.
    /// Presentations retain this role in movable and touch layouts without
    /// depending on a keyboard shortcut.
    CloseControl,
    /// The resize control identifies a host-controlled window sizing surface.
    /// Example systems clamp its dimensions and position to the available
    /// viewport while preserving the input region.
    ResizeControl,
}

/// Visibility and focus modes supported by the single interactive console.
/// Closed presentation has no editing ownership; open presentation can release
/// or acquire keyboard focus.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleMode {
    /// The console is closed and does not own keyboard editing. The default
    /// presentation hides its root while leaving the visible open control
    /// available.
    #[default]
    Closed,
    /// The console is visible but has released its editor focus. Game input can
    /// continue until the player deliberately acquires console editing focus
    /// again.
    OpenReleased,
    /// The console is visible and owns its editable input focus. Cooperating
    /// game input systems use the public focus gate to defer their keyboard
    /// handling.
    OpenEditing,
}

/// Tracks requested console visibility and the focus target saved before
/// opening. The state contains no camera or window and is shared with
/// replacement presentations.
#[derive(Debug, Default, Clone, Copy, Resource)]
pub struct ConsoleState {
    /// Current visibility and editing ownership.
    mode: ConsoleMode,
    /// Prior focus restored only when its entity still exists.
    previous_focus: Option<Entity>,
    /// Latest visibility action awaiting the UI adapter.
    request: Option<VisibilityRequest>,
    /// Frame-local ownership prevents a closing key from reaching cooperating
    /// gameplay.
    keyboard_owned: Option<KeyboardOwnership>,
}

/// Presence marker records input ownership independently of the applied
/// visibility mode.
#[derive(Debug, Clone, Copy)]
struct KeyboardOwnership;

/// Allows cooperating gameplay keyboard systems only after console input has
/// released ownership.
/// Order the system after `PromptlineSystems::Input`; a closing key remains
/// consumed for that entire frame.
#[must_use]
pub fn gameplay_input_allowed(state: Res<'_, ConsoleState>) -> bool {
    !state.is_editing() && state.keyboard_owned.is_none()
}

impl ConsoleState {
    /// Requests an open console with editing focus acquired by the UI adapter.
    /// Applications can invoke this action without using a particular key or
    /// pointer control.
    pub const fn open(&mut self) {
        self.request = Some(VisibilityRequest::Open);
    }

    /// Requests a closed console and releases its editing ownership. The UI
    /// adapter restores a valid prior focus target when it applies this
    /// request.
    pub const fn close(&mut self) {
        self.request = Some(VisibilityRequest::Close);
    }

    /// Shows the console without acquiring new keyboard editing ownership from
    /// another widget.
    /// Existing editor focus remains intact; closed presentation becomes
    /// visible with its current focus state.
    pub const fn show(&mut self) {
        self.request = Some(VisibilityRequest::Show);
    }

    /// Releases editing focus while keeping the console's input, output, and
    /// help visible.
    /// A valid saved focus target is restored before cooperating game input
    /// resumes on another frame.
    pub const fn release_focus(&mut self) {
        self.request = Some(VisibilityRequest::Release);
    }

    /// Reports whether the console currently owns keyboard editing focus.
    /// Cooperating game systems use this state rather than independently
    /// inferring ownership from visibility.
    #[must_use]
    pub const fn is_editing(&self) -> bool {
        matches!(self.mode, ConsoleMode::OpenEditing)
    }

    /// Returns the current visibility and focus mode without changing it.
    /// Presentations derive their animation or layout from this state rather
    /// than storing another authority.
    #[must_use]
    pub const fn mode(&self) -> ConsoleMode {
        self.mode
    }
}

impl Plugin for PromptlineUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<crate::PromptlineRuntimePlugin>() {
            app.add_plugins(crate::PromptlineRuntimePlugin);
        }
        // Initialize only console state; the host retains camera and window ownership.
        app.init_resource::<ConsoleState>()
            .init_resource::<InputFocus>()
            .init_resource::<ConsoleBindings>()
            .init_resource::<ConsoleTheme>()
            .init_resource::<SeverityColors>()
            .init_resource::<HighlightColors>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<bevy::input::keyboard::Key>>()
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .init_resource::<EditorIntent>()
            .init_resource::<UiAnalysis>()
            .init_resource::<UiCycle>()
            .init_resource::<UiHistory>();
        // Editor scheduling and presentation remain separate from retained output scrolling.
        configure_editor(app);
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, browser::ownership.after(PromptlineSystems::Input));
        configure_presentation(app);
        scrollback::install(app);
        highlight::install(app);
        // Bevy 0.20 routes editing input only to editors that also carry `TextInput`.
        app.add_observer(attach_input_to_editor)
            .add_observer(attach_input_to_role);
    }

    fn finish(&self, app: &mut App) {
        keyboard::install(app);
        highlight::install_render(app);
    }
}

/// Console editors keep Bevy's keyboard, pointer, and IME widget behavior.
fn attach_input_to_editor(
    event: On<'_, '_, Add<EditableText>>,
    roles: Query<'_, '_, (), With<ConsoleRole>>,
    mut commands: Commands<'_, '_>,
) {
    if roles.contains(event.entity) {
        commands.entity(event.entity).insert_if_new(TextInput);
    }
}

/// A role assigned after its editor receives the same widget behavior.
fn attach_input_to_role(
    event: On<'_, '_, Insert<ConsoleRole>>,
    editors: Query<'_, '_, (), (With<EditableText>, Without<TextInput>)>,
    mut commands: Commands<'_, '_>,
) {
    if editors.contains(event.entity) {
        commands.entity(event.entity).insert(TextInput);
    }
}

/// Orders editing after Bevy's applied text and before the command runtime
/// dispatch.
fn configure_editor(app: &mut App) {
    // Console ownership precedes cooperating gameplay input systems.
    app.add_systems(
        PreUpdate,
        keyboard::forward_shift_space
            .after(bevy::input_focus::InputFocusSystems::Dispatch)
            .run_if(keyboard::is_legacy_dispatch),
    );
    app.add_systems(Startup, spawn_console)
        .configure_sets(
            PostUpdate,
            (
                PromptlineSystems::Analysis,
                PromptlineSystems::Dispatch,
                PromptlineSystems::Output,
                PromptlineSystems::Presentation,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                sync_editor_focus,
                capture_console_keys,
                apply_visibility_request,
                present_visibility,
                theme::apply_theme,
            )
                .chain()
                .in_set(PromptlineSystems::Input),
        )
        .add_systems(
            PostUpdate,
            (apply_completion, apply_history, submit_editor)
                .chain()
                .in_set(PromptlineSystems::Analysis)
                .after(EditableTextSystems),
        );
}

/// Orders runtime observations before content rendering and laid-out ghost
/// placement.
fn configure_presentation(app: &mut App) {
    // Content follows dispatch; ghost coordinates follow the host's actual UI layout.
    app.add_systems(
        PostUpdate,
        apply_submission_result.in_set(PromptlineSystems::Output),
    )
    .add_systems(
        PostUpdate,
        (refresh_analysis, render_console_text)
            .chain()
            .in_set(PromptlineSystems::Presentation),
    )
    .add_systems(
        PostUpdate,
        position_ghost.after(bevy::ui::UiSystems::Layout),
    );
}

/// Synchronizes visible editing ownership before console and cooperating
/// gameplay keys are read.
fn sync_editor_focus(
    mut state: ResMut<'_, ConsoleState>,
    focus: Res<'_, InputFocus>,
    bindings: Res<'_, ConsoleBindings>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    roles: Query<'_, '_, (Entity, &ConsoleRole)>,
) {
    // Closed consoles leave other widgets' focus unchanged.
    if state.mode == ConsoleMode::Closed {
        return;
    }
    let input = roles
        .iter()
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity));
    let is_editor_focused = input.is_some() && focus.get() == input;
    // Bevy 0.20 text input clears focus on Escape before this system runs.
    // When that key is also the close binding, editing remains owned so the
    // close action restores the saved focus instead of only releasing it.
    let is_editing_before = state.mode == ConsoleMode::OpenEditing;
    let is_unfocused = focus.get().is_none();
    let is_close_pressed = keys.just_pressed(bindings.close);
    if is_editing_before && is_unfocused && is_close_pressed {
        return;
    }
    // Open visibility persists when another widget takes editing ownership.
    state.mode = if is_editor_focused {
        ConsoleMode::OpenEditing
    } else {
        ConsoleMode::OpenReleased
    };
}

/// A requested visibility action, distinct from applied editor ownership.
#[derive(Debug, Clone, Copy)]
enum VisibilityRequest {
    /// Open and acquire editing focus when an input role exists.
    Open,
    /// Close and restore the saved focus when its entity remains valid.
    Close,
    /// Show without taking new focus from another widget.
    Show,
    /// Restore prior focus while retaining visible console presentation.
    Release,
}

/// Builds public roles with Bevy UI components and no camera or window
/// allocation.
fn spawn_console(mut commands: Commands<'_, '_>) {
    // Input and output roles remain independently replaceable by the host.
    let root = spawn_root(commands.reborrow());
    let output = spawn_output(commands.reborrow());
    let input = spawn_input(commands.reborrow());
    let candidates = spawn_text_role(commands.reborrow(), ConsoleRole::Candidates, 36.0);
    let help = spawn_text_role(commands.reborrow(), ConsoleRole::Help, 54.0);
    // The ghost remains a sibling rather than part of the editable value.
    let ghost = spawn_ghost(commands.reborrow());
    commands
        .entity(root)
        .add_children(&[output, input, candidates, help, ghost]);
    spawn_open_button(commands.reborrow());
}

/// Spawns the default root role through ordinary Bevy UI components.
fn spawn_root(mut commands: Commands<'_, '_>) -> Entity {
    commands
        .spawn((
            ConsoleRole::Root,
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                bottom: px(0),
                left: px(0),
                width: percent(100),
                height: px(360),
                max_height: percent(92),
                flex_direction: FlexDirection::Column,
                padding: px(16).all(),
                row_gap: px(8),
                ..default()
            },
            BackgroundColor(Color::srgba(0.025, 0.035, 0.045, 0.97)),
            GlobalZIndex(100),
        ))
        .id()
}

/// Spawns the default output role through ordinary Bevy UI components.
fn spawn_output(mut commands: Commands<'_, '_>) -> Entity {
    // Bevy clips descendants; the viewport must enclose the rendered text.
    let viewport = commands
        .spawn(Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_height: px(32),
            overflow: Overflow::scroll(),
            ..default()
        })
        .id();
    let text = commands
        .spawn((
            ConsoleRole::Output,
            Text::new(""),
            scrollback::OutputContent::default(),
            TextLayout::linebreak(LineBreak::WordOrCharacter),
            text_font(),
            TextColor(Color::srgb(0.88, 0.93, 0.94)),
            Node {
                width: percent(100),
                flex_shrink: 0.0,
                ..default()
            },
        ))
        .id();
    commands.entity(viewport).add_child(text);
    scrollback::decorate(commands.reborrow(), viewport);
    viewport
}

/// Spawns the default input role through ordinary Bevy UI components.
fn spawn_input(mut commands: Commands<'_, '_>) -> Entity {
    commands
        .spawn((
            ConsoleRole::Input,
            Node {
                width: percent(100),
                height: px(30),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            EditableText {
                allow_newlines: false,
                max_characters: Some(4096),
                ..default()
            },
            TextLayout::no_wrap(),
            text_font(),
            TextColor(Color::srgb(0.92, 0.97, 0.97)),
            TextCursorStyle {
                color: Color::WHITE,
                selection_color: Color::srgb(0.15, 0.35, 0.45),
                ..default()
            },
        ))
        .id()
}

/// Spawns the default ghost role through ordinary Bevy UI components.
fn spawn_ghost(mut commands: Commands<'_, '_>) -> Entity {
    commands
        .spawn((
            ConsoleRole::Ghost,
            Text::new(""),
            text_font(),
            TextColor(Color::srgb(0.62, 0.8, 0.78)),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id()
}

/// Creates the visible access control independently of console visibility.
fn spawn_open_button(mut commands: Commands<'_, '_>) {
    commands
        .spawn((
            ConsoleRole::OpenControl,
            bevy::ui_widgets::Button,
            Node {
                position_type: PositionType::Absolute,
                left: px(16),
                bottom: px(16),
                padding: px(10).all(),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.15, 0.18)),
            GlobalZIndex(101),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Open console"),
                text_font(),
                TextColor(Color::WHITE),
            ));
        })
        .observe(open_control);
}

/// Uses the host's font source with an explicit readable console text size.
fn text_font() -> TextFont {
    TextFont {
        font_size: FontSize::Px(18.0),
        ..default()
    }
}

/// Reserves contextual content height independently from the input layout.
fn spawn_text_role(mut commands: Commands<'_, '_>, role: ConsoleRole, height: f32) -> Entity {
    commands
        .spawn((
            role,
            Text::new(""),
            text_font(),
            TextColor(Color::srgb(0.76, 0.88, 0.88)),
            Node {
                height: px(height),
                min_height: px(height),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id()
}

/// Opens the console through a visible pointer and touch control.
fn open_control(_: On<'_, '_, PointerClick>, mut state: ResMut<'_, ConsoleState>) {
    state.open();
}

/// Applies focus ownership only after an explicit visibility request or focus
/// change.
fn apply_visibility_request(
    mut state: ResMut<'_, ConsoleState>,
    mut focus: ResMut<'_, InputFocus>,
    roles: Query<'_, '_, (Entity, &ConsoleRole)>,
    entities: Query<'_, '_, Entity>,
) {
    // Resolve the public input role rather than retaining a private entity handle.
    let input = roles
        .iter()
        .find_map(|(entity, role)| (*role == ConsoleRole::Input).then_some(entity));
    match state.request.take() {
        Some(VisibilityRequest::Open) => {
            open_editor(&mut state, &mut focus, input);
        }
        Some(VisibilityRequest::Close) => {
            // Despawned restoration targets cannot regain focus.
            let previous = state
                .previous_focus
                .filter(|entity| entities.contains(*entity));
            close_editor(&mut state, &mut focus, input, previous);
        }
        Some(VisibilityRequest::Show) => {
            let is_focused = input.is_some() && focus.get() == input;
            state.mode = if is_focused {
                ConsoleMode::OpenEditing
            } else {
                ConsoleMode::OpenReleased
            };
        }
        Some(VisibilityRequest::Release) => {
            let previous = state
                .previous_focus
                .filter(|entity| entities.contains(*entity));
            close_editor(&mut state, &mut focus, input, previous);
            state.mode = ConsoleMode::OpenReleased;
        }
        None => {
            if state.mode == ConsoleMode::OpenEditing && focus.get() != input {
                // Missing input leaves the console visible without taking editing ownership.
                state.mode = ConsoleMode::OpenReleased;
            }
        }
    }
}

/// Acquires a valid input while preserving focus saved by the first opening.
fn open_editor(state: &mut ConsoleState, focus: &mut InputFocus, input: Option<Entity>) {
    let current = focus.get();
    // Repeated opening keeps the original restoration target.
    if current != input {
        state.previous_focus = current;
    }
    // Missing input leaves visibility available without editing ownership.
    state.mode = ConsoleMode::OpenReleased;
    if let Some(input) = input {
        focus.set(input, FocusCause::Navigated);
        state.mode = ConsoleMode::OpenEditing;
    }
}

/// Closing preserves other widgets' focus and restores only a valid saved
/// target.
fn close_editor(
    state: &mut ConsoleState,
    focus: &mut InputFocus,
    input: Option<Entity>,
    previous: Option<Entity>,
) {
    let is_focus_owned = input.is_some() && focus.get() == input;
    // A released console has no authority to clear another widget's focus.
    if is_focus_owned {
        focus.clear();
        state.previous_focus = None;
        if let Some(previous) = previous {
            focus.set(previous, FocusCause::Navigated);
        }
    }
    // Closing visibility does not depend on whether editing focus was owned.
    state.mode = ConsoleMode::Closed;
}

/// Derives default visibility from the applied mode, leaving styling externally
/// replaceable.
fn present_visibility(
    state: Res<'_, ConsoleState>,
    mut nodes: Query<'_, '_, (&ConsoleRole, &mut Node)>,
) {
    // Visibility derives from applied state rather than a second independent flag.
    let is_closed = state.mode == ConsoleMode::Closed;
    for (role, mut node) in &mut nodes {
        match role {
            ConsoleRole::Root => {
                node.display = if is_closed {
                    Display::None
                } else {
                    Display::Flex
                }
            }
            // The access control remains visible when the console itself is closed.
            ConsoleRole::OpenControl => {
                node.display = if is_closed {
                    Display::Flex
                } else {
                    Display::None
                }
            }
            ConsoleRole::Input
            | ConsoleRole::Output
            | ConsoleRole::Help
            | ConsoleRole::Ghost
            | ConsoleRole::Candidates
            | ConsoleRole::DragRegion
            | ConsoleRole::CloseControl
            | ConsoleRole::ResizeControl => {}
        }
    }
}

/// An explicit submit request awaiting the completed editor update.
#[derive(Debug, Default, Resource)]
struct EditorIntent {
    /// A submit key was pressed outside composition.
    submit: Option<SubmitIntent>,
    /// Explicit completion requested by the player.
    completion: Option<CompletionDirection>,
    /// Explicit history navigation requested outside composition.
    history: Option<HistoryDirection>,
    /// Draft snapshot associated with this UI submission.
    pending: Option<crate::command::SubmittedText>,
}

/// A history action preserves the distinction between older and newer input.
#[derive(Debug, Clone, Copy)]
enum HistoryDirection {
    /// Recall an older accepted command.
    Older,
    /// Recall a newer command or restore the saved draft.
    Newer,
}

/// Presence marker for a submission action rather than a persistent boolean
/// mode.
#[derive(Debug, Clone, Copy)]
struct SubmitIntent;

/// Captures console keys while preserving upstream editor ownership and
/// composition.
fn capture_console_keys(
    bindings: Res<'_, ConsoleBindings>,
    keys: Res<'_, ButtonInput<KeyCode>>,
    mut state: ResMut<'_, ConsoleState>,
    mut intent: ResMut<'_, EditorIntent>,
    mut editors: Query<'_, '_, (&ConsoleRole, &mut EditableText)>,
) {
    // Preserve ownership of this frame even when its close action releases focus.
    state.keyboard_owned = state.is_editing().then_some(KeyboardOwnership);
    // Visibility actions precede text actions and never submit the toggle glyph.
    if bindings.toggle.is_some_and(|key| keys.just_pressed(key)) {
        state.keyboard_owned = Some(KeyboardOwnership);
        toggle_console(&mut state, editors.reborrow(), bindings.toggle);
        return;
    }
    if !state.is_editing() {
        return;
    }
    if keys.just_pressed(bindings.close) {
        state.close();
        return;
    }
    // Both submission and completion defer to active or queued composition.
    let mut input_editors = editors
        .iter()
        .filter(|(role, _)| **role == ConsoleRole::Input);
    let is_composing = input_editors.any(|(_, editor)| has_composition(editor));
    if is_composing {
        return;
    }
    // Only focused, non-composing input can create a text action.
    capture_edit_actions(&keys, &bindings, &mut intent);
}

/// Requests visibility while excluding only this toggle key's queued glyph.
fn toggle_console(
    state: &mut ConsoleState,
    mut editors: Query<'_, '_, (&ConsoleRole, &mut EditableText)>,
    toggle: Option<KeyCode>,
) {
    if state.mode == ConsoleMode::Closed {
        state.open();
    } else {
        state.close();
        // Other queued edits remain owned by the upstream editor.
        editors
            .iter_mut()
            .filter(|(role, _)| **role == ConsoleRole::Input)
            .for_each(|(_, mut editor)| {
                if toggle == Some(KeyCode::Backquote) {
                    editor.pending_edits.retain(|edit| !is_toggle_glyph(edit));
                }
            });
    }
}

/// Matches exactly one queued toggle character without treating text as a path.
fn is_toggle_glyph(edit: &TextEdit) -> bool {
    matches!(edit, TextEdit::Insert(value) if value == "`" || value.chars().eq(std::iter::once('~')))
}

/// Active and queued composition both take priority over console text actions.
fn has_composition(editor: &EditableText) -> bool {
    editor.is_composing()
        || editor.pending_edits.iter().any(|edit| {
            matches!(
                edit,
                TextEdit::ImeSetCompose { .. } | TextEdit::ImeCommit { .. }
            )
        })
}

/// Converts explicit text-action keys into frame-local editor intentions.
fn capture_edit_actions(
    keys: &ButtonInput<KeyCode>,
    bindings: &ConsoleBindings,
    intent: &mut EditorIntent,
) {
    // History navigation remains independent of command submission.
    if keys.just_pressed(bindings.previous) {
        intent.history = Some(HistoryDirection::Older);
    } else if keys.just_pressed(bindings.next) {
        intent.history = Some(HistoryDirection::Newer);
    }
    if keys.just_pressed(bindings.submit) {
        intent.submit = Some(SubmitIntent);
    }
    // Shift reverses completion without creating a command submission.
    if keys.just_pressed(bindings.complete) {
        intent.completion = Some(
            if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
                CompletionDirection::Backward
            } else {
                CompletionDirection::Forward
            },
        );
    }
}

/// Sends actual applied text before the runtime's post-update dispatch
/// boundary.
fn submit_editor(
    mut intent: ResMut<'_, EditorIntent>,
    editors: Query<'_, '_, (&ConsoleRole, &EditableText)>,
    submissions: Option<ResMut<'_, Messages<crate::SubmitCommand>>>,
) {
    // Consume one explicit submit action after upstream edits have applied.
    if intent.submit.take().is_none() {
        return;
    }
    let Some((_, editor)) = editors
        .iter()
        .find(|(role, _)| **role == ConsoleRole::Input)
    else {
        return;
    };
    // Preserve ordering while an asynchronous paste still precedes submission.
    if editor.pending_paste.is_some() {
        intent.submit = Some(SubmitIntent);
        return;
    }
    let Some(mut submissions) = submissions else {
        return;
    };
    // Queue and pending-clear snapshot need independent ownership of this bounded draft.
    let text = editor.value().to_string();
    let mut submission = crate::SubmitCommand::new(text.clone());
    submission.source = crate::command::SubmissionSource::Ui;
    submissions.write(submission);
    intent.pending = Some(crate::command::SubmittedText(text));
}

/// Clears accepted UI input while preserving a rejected or subsequently edited
/// draft.
fn apply_submission_result(
    mut notifications: MessageReader<'_, '_, crate::CommandNotification>,
    mut intent: ResMut<'_, EditorIntent>,
    mut editors: Query<'_, '_, (&ConsoleRole, &mut EditableText)>,
) {
    // Programmatic submissions cannot clear the authoritative UI draft.
    for notification in notifications.read() {
        if notification.source != crate::command::SubmissionSource::Ui {
            continue;
        }
        match &notification.status {
            crate::command::InvocationStatus::Accepted => {
                // Acceptance clears only the unchanged snapshot belonging to this UI request.
                if let Some(snapshot) = intent.pending.take() {
                    clear_accepted_edit(editors.reborrow(), &snapshot.0);
                }
            }
            // Rejection releases the pending snapshot while leaving the editor text intact.
            crate::command::InvocationStatus::Rejected(_) => {
                intent.pending = None;
            }
            crate::command::InvocationStatus::Submitted
            | crate::command::InvocationStatus::Started
            | crate::command::InvocationStatus::Finished(_) => {}
        }
    }
}

/// Clears only the input role whose actual value still matches accepted UI
/// input.
fn clear_accepted_edit(
    mut editors: Query<'_, '_, (&ConsoleRole, &mut EditableText)>,
    snapshot: &str,
) {
    // An independently edited draft must survive an older invocation's acceptance.
    editors
        .iter_mut()
        .filter(|(role, _)| **role == ConsoleRole::Input)
        .for_each(|(_, mut editor)| {
            let is_unchanged = editor.value() == snapshot;
            if is_unchanged {
                editor.clear();
            }
        });
}

use crate::completion::CompletionDirection;

/// Derived text for public presentation roles, keeping draft data out of
/// diagnostics.
#[derive(Debug, Default, Resource)]
struct UiAnalysis {
    /// Candidate spellings shown near the editable input.
    candidates: String,
    /// Metadata description shown in reserved help space.
    help: String,
    /// Non-authoritative completion suffix.
    ghost: String,
}

/// Reads the actual editable value and caret without retaining a World borrow.
fn editor_snapshot(
    world: &mut World,
) -> Option<(Entity, String, usize, crate::completion::EditingMode)> {
    let mut query = world.query::<(Entity, &ConsoleRole, &EditableText)>();
    query
        .iter(world)
        .find(|(_, role, _)| **role == ConsoleRole::Input)
        .map(|(entity, _, editor)| {
            let text = editor.value().to_string();
            let selection = editor.editor.raw_selection();
            let caret = selection.focus().index().min(text.len());
            let mode = if editor.is_composing() {
                crate::completion::EditingMode::Composition
            } else if selection.is_collapsed() {
                crate::completion::EditingMode::Caret
            } else {
                crate::completion::EditingMode::Selection
            };
            (entity, text, caret, mode)
        })
}

/// Frozen UI completion session tied to the registry and resulting editor
/// position.
#[derive(Debug, Default, Resource)]
struct UiCycle {
    /// Current session, if metadata and the authoritative draft still match.
    session: Option<crate::completion::CompletionCycle>,
    /// Registry metadata revision at the last completion step.
    revision: u64,
}

/// Applies an explicit step through the same frozen candidate list until input
/// changes.
fn apply_completion(world: &mut World) {
    // A preview alone cannot enter this explicit completion boundary.
    let direction = world
        .get_resource_mut::<EditorIntent>()
        .and_then(|mut intent| intent.completion.take());
    let Some(direction) = direction else {
        return;
    };
    let Some((entity, text, caret, _)) = editor_snapshot(world) else {
        return;
    };
    // Registry changes invalidate candidates independently of text and caret movement.
    let revision = world
        .get_resource::<crate::CommandRegistry>()
        .map_or(0, crate::CommandRegistry::revision);
    let mut cycle = world.remove_resource::<UiCycle>().unwrap_or_default();
    let is_draft_current = cycle
        .session
        .as_ref()
        .is_some_and(|session| session.is_current(&text, caret));
    let is_current = cycle.revision == revision && is_draft_current;
    // User edits, caret moves, and registry changes replace the frozen session.
    if !is_current {
        cycle.session = crate::completion::analyze(world, &text, caret)
            .map(crate::completion::Analysis::into_cycle)
            .ok();
        cycle.revision = revision;
    }
    let completion = cycle
        .session
        .as_mut()
        .and_then(|session| session.advance(&text, direction).ok());
    // Apply only a successfully selected replacement, including its actual byte caret.
    world.insert_resource(cycle);
    if let Some(completion) = completion {
        replace_editor(world, entity, &completion.text, completion.caret);
    }
}

/// Last history replacement distinguishes continued browsing from external
/// edits.
#[derive(Debug, Default, Resource)]
struct UiHistory {
    /// Snapshot contains command input and therefore uses redacted diagnostic
    /// formatting.
    expected: Option<crate::command::SubmittedText>,
}

/// Applies explicit history navigation after the editor has processed text
/// changes.
fn apply_history(world: &mut World) {
    // A preview alone cannot enter this explicit completion boundary.
    let direction = world
        .get_resource_mut::<EditorIntent>()
        .and_then(|mut intent| intent.history.take());
    let Some(direction) = direction else {
        return;
    };
    let Some((entity, text, _, mode)) = editor_snapshot(world) else {
        return;
    };
    if mode == crate::completion::EditingMode::Composition {
        return;
    }
    // Composition remains authoritative; history never competes with its preview.
    let mut browsing = world.remove_resource::<UiHistory>().unwrap_or_default();
    let replacement = world
        .get_resource_mut::<crate::command::CommandHistory>()
        .and_then(|mut history| {
            // External edits begin a new browsing session with their own saved draft.
            if browsing
                .expected
                .as_ref()
                .is_none_or(|expected| expected.0 != text)
            {
                history.stop_browsing();
            }
            match direction {
                HistoryDirection::Older => history.previous(&text),
                HistoryDirection::Newer => history.next(),
            }
            .map(str::to_owned)
        });
    if let Some(replacement) = replacement {
        replace_editor(world, entity, &replacement, replacement.len());
        browsing.expected = Some(crate::command::SubmittedText(replacement));
    }
    // Retain the expected replacement to distinguish browsing from a later manual edit.
    world.insert_resource(browsing);
}

/// Uses the upstream driver to place the caret after a completed token.
fn replace_editor(world: &mut World, entity: Entity, text: &str, caret: usize) {
    if world.contains_resource::<FontCx>() && world.contains_resource::<LayoutCx>() {
        world.resource_scope(|world, mut fonts: Mut<'_, FontCx>| {
            world.resource_scope(|world, mut layouts: Mut<'_, LayoutCx>| {
                if let Some(mut editor) = world.get_mut::<EditableText>(entity) {
                    editor.editor.set_text(text);
                    editor
                        .editor
                        .driver(&mut fonts.context, &mut layouts.0)
                        .move_to_byte(caret);
                }
            });
        });
    } else if let Some(mut editor) = world.get_mut::<EditableText>(entity) {
        // Headless role construction has no font contexts; the host applies navigation later.
        editor.editor.set_text(text);
        editor.queue_edit(TextEdit::TextEnd(false));
    }
}

/// Builds live metadata views without invoking command argument validators.
fn refresh_analysis(world: &mut World) {
    let Some((_, text, caret, mode)) = editor_snapshot(world) else {
        clear_analysis(world);
        return;
    };
    // Discovery reads metadata without invoking argument validators or command handlers.
    let analysis = match crate::completion::analyze(world, &text, caret) {
        Ok(analysis) => analysis,
        Err(error) => {
            // Invalid input drops stale discovery while retaining readable local guidance.
            clear_analysis(world);
            if let Some(mut view) = world.get_resource_mut::<UiAnalysis>() {
                view.help = error.to_string();
            }
            return;
        }
    };
    let candidates = analysis
        .candidates()
        .iter()
        .map(|candidate| candidate.value.as_str())
        .take(8)
        .collect::<Vec<_>>()
        .join("  ");
    // Reserved help remains available after the caret leaves a command candidate.
    let help = analysis.diagnostic().map_or_else(
        || {
            if analysis.help().is_empty() {
                crate::completion::active_command_help(world, &text)
            } else {
                analysis.help().to_owned()
            }
        },
        |diagnostic| diagnostic.to_string(),
    );
    let ghost = analysis.ghost(mode).unwrap_or_default().to_owned();
    // Derived views stay separate from submission, copying, and accepted-command history.
    if let Some(mut view) = world.get_resource_mut::<UiAnalysis>() {
        view.candidates = candidates;
        view.help = help;
        view.ghost = ghost;
    }
    highlight::retain_analysis(world, analysis);
}

/// Invalid or missing input cannot retain help or candidates from an older
/// permitted draft.
fn clear_analysis(world: &mut World) {
    highlight::clear_analysis(world);
    if let Some(mut view) = world.get_resource_mut::<UiAnalysis>() {
        *view = UiAnalysis::default();
    }
}

/// Updates bounded output and derived text without adding ghost text to the
/// editor.
fn render_console_text(
    view: Res<'_, UiAnalysis>,
    mut texts: Query<'_, '_, (&ConsoleRole, &mut Text)>,
) {
    // Public roles select content without assigning execution authority to displayed text.
    for (role, mut text) in &mut texts {
        let value = match role {
            ConsoleRole::Help => view.help.clone(),
            ConsoleRole::Candidates => view.candidates.clone(),
            ConsoleRole::Ghost => view.ghost.clone(),
            ConsoleRole::Output
            | ConsoleRole::Root
            | ConsoleRole::Input
            | ConsoleRole::OpenControl
            | ConsoleRole::DragRegion
            | ConsoleRole::CloseControl
            | ConsoleRole::ResizeControl => continue,
        };
        // Avoid component writes when its displayed content has not changed.
        if text.0 != value {
            text.0 = value;
        }
    }
}

/// Positions the preview from Bevy's laid-out caret rectangle in physical
/// pixels.
fn position_ghost(
    geometry: Query<
        '_,
        '_,
        (
            &ConsoleRole,
            &ComputedNode,
            &UiGlobalTransform,
            Option<&bevy::text::TextLayoutInfo>,
            Option<&EditableText>,
        ),
    >,
    mut ghosts: Query<'_, '_, (&ConsoleRole, &mut Node)>,
) {
    let root = geometry
        .iter()
        .find(|(role, ..)| **role == ConsoleRole::Root);
    let input = geometry
        .iter()
        .find(|(role, ..)| **role == ConsoleRole::Input);
    let (
        Some((_, root_node, root_transform, _, _)),
        Some((_, input_node, input_transform, Some(layout), scroll)),
    ) = (root, input)
    else {
        return;
    };
    // Use the shaped caret rectangle instead of estimating character widths.
    let Some((_, cursor)) = layout.cursor else {
        return;
    };
    // Translate physical caret geometry into the root coordinate system before scaling.
    let offset = input_transform.translation - root_transform.translation
        + (root_node.size - input_node.size) * 0.5;
    let scroll = scroll.map_or(Vec2::ZERO, |editor| editor.viewport.offset);
    // Scroll and inverse UI scale convert physical pixel coordinates into logical placement.
    let position =
        (offset + Vec2::new(cursor.max.x, cursor.min.y) - scroll) * root_node.inverse_scale_factor;
    for (role, mut node) in &mut ghosts {
        if *role == ConsoleRole::Ghost {
            node.left = px(position.x);
            node.top = px(position.y);
        }
    }
}
