//! Scrollback follows live output until explicit review and counts arrivals.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_promptline::command::output::OutputView;
use bevy_promptline::{
    Output,
    input::{ConsoleRole, PromptlineUiPlugin},
};

/// An absent presentation cannot discard retained output during replacement.
#[test]
fn output_survives_a_temporarily_missing_text_component() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.world_mut().resource_mut::<Output>().write("old");
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    app.world_mut().entity_mut(entity).remove::<Text>();
    app.world_mut().resource_mut::<Output>().write("arrival");
    app.update();
    assert_eq!(
        app.world().resource::<Output>().texts().collect::<Vec<_>>(),
        ["old", "arrival"]
    );
    app.world_mut().entity_mut(entity).insert(Text::default());
    app.update();
    assert_eq!(rendered_output(&mut app, entity), "old\narrival");
}

#[test]
fn review_counts_entries_without_conflating_them_with_wrapped_lines() {
    let mut output = Output::default();
    let mut view = OutputView::default();
    assert!(view.is_following());
    view.pause_following(&output);
    assert!(view.is_following());
    output.write("first");
    view.pause_following(&output);
    assert!(!view.is_following());
    output.write("second\nthird");
    assert_eq!(view.unread_entries(&output), 1);
    view.follow_latest();
    assert!(view.is_following());
    assert_eq!(view.unread_entries(&output), 0);
}

#[test]
fn clearing_or_replacing_output_resets_a_stale_review_session() {
    let mut output = Output::default();
    output.write("old");
    let mut view = OutputView::default();
    view.pause_following(&output);
    output.clear();
    view.synchronize(&output);
    assert!(view.is_following());
    output.write("one");
    output.write("two");
    view.pause_following(&output);
    output = Output::default();
    output.write("replacement");
    view.synchronize(&output);
    assert!(view.is_following());
    assert_eq!(view.unread_entries(&output), 0);
}

#[test]
fn viewport_can_review_every_retained_entry_instead_of_a_fixed_tail() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    for index in 0..20 {
        app.world_mut()
            .resource_mut::<Output>()
            .write(format!("row {index}"));
    }
    app.update();
    let output = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let text = rendered_output(&mut app, output);
    assert!(text.starts_with("row 0\n"));
    assert!(text.ends_with("row 19"));
}

/// Bevy's reader concatenates the root and styled sections in actual display order.
fn rendered_output(app: &mut App, entity: Entity) -> String {
    app.world_mut()
        .run_system_once(move |mut reader: TextUiReader<'_, '_>| {
            reader
                .iter(entity)
                .map(|(_, _, text, ..)| text)
                .collect::<String>()
        })
        .unwrap()
}

/// Replacement editors share the retained transcript and stay untouched while idle.
#[test]
fn selectable_output_receives_retained_text_after_replacing_an_existing_role() {
    use bevy::text::EditableText;

    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    for index in 0..30 {
        app.world_mut()
            .resource_mut::<Output>()
            .write(format!("row {index} 日本語"));
    }
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let old_sections = app.world().get::<Children>(entity).unwrap().to_vec();
    app.world_mut()
        .entity_mut(entity)
        .remove::<Text>()
        .insert(EditableText::default());
    app.update();
    assert!(
        old_sections
            .into_iter()
            .all(|section| app.world().get_entity(section).is_err())
    );
    let editor = app.world().get::<EditableText>(entity).unwrap();
    assert!(editor.editor.raw_text().starts_with("row 0 日本語\n"));
    assert!(editor.editor.raw_text().ends_with("row 29 日本語"));
    let changed = app
        .world_mut()
        .query::<Ref<'_, EditableText>>()
        .get(app.world(), entity)
        .unwrap()
        .last_changed();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<Ref<'_, EditableText>>()
            .get(app.world(), entity)
            .unwrap()
            .last_changed(),
        changed
    );
    app.world_mut().resource_mut::<Output>().write("arrival");
    app.update();
    assert!(
        app.world()
            .get::<EditableText>(entity)
            .unwrap()
            .editor
            .raw_text()
            .ends_with("row 29 日本語\narrival")
    );
}
