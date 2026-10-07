//! Default output keeps source text while styling retained entry severity.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use bevy_promptline::input::{
    ConsoleRole, ConsoleTheme, PromptlineUiPlugin, highlight::SeverityColors,
};
use bevy_promptline::{Output, Severity};

/// Reassigned roles retain their presentation when output and its palette change.
#[test]
fn reassigned_role_does_not_receive_output_text_or_severity_colors() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.world_mut()
        .resource_mut::<Output>()
        .push(Severity::Warn, "test", "retained".into());
    app.update();
    let root = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let original_color = app.world().get::<TextColor>(root).unwrap().0;
    app.world_mut()
        .entity_mut(root)
        .insert(ConsoleRole::DragRegion);
    app.world_mut().resource_mut::<SeverityColors>().warn = Some(Color::WHITE);
    app.world_mut()
        .resource_mut::<Output>()
        .push(Severity::Error, "test", "new error".into());
    app.update();
    assert_eq!(app.world().get::<Text>(root).unwrap().0, "retained");
    assert_eq!(
        app.world().get::<TextColor>(root).unwrap().0,
        original_color
    );
}

#[test]
fn output_sections_preserve_text_and_use_configured_severity_colors() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.insert_resource(SeverityColors {
        debug: Some(Color::srgb(0.2, 0.4, 0.6)),
        warn: Some(Color::srgb(0.8, 0.6, 0.2)),
        error: Some(Color::srgb(0.9, 0.2, 0.3)),
    });
    for (severity, text) in [
        (Severity::Info, "info"),
        (Severity::Warn, "warn"),
        (Severity::Error, "error"),
        (Severity::Debug, "debug"),
        (Severity::Trace, "trace"),
    ] {
        app.world_mut()
            .resource_mut::<Output>()
            .push(severity, "test", text.into());
    }
    app.update();
    let root = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let sections = app
        .world_mut()
        .run_system_once(move |mut reader: TextUiReader<'_, '_>| {
            reader
                .iter(root)
                .map(|(_, _, text, _, color, ..)| (text.to_owned(), color))
                .collect::<Vec<_>>()
        })
        .unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|(text, _)| text.as_str())
            .collect::<String>(),
        "info\nwarn\nerror\ndebug\ntrace"
    );
    assert_eq!(
        sections.iter().map(|(_, color)| *color).collect::<Vec<_>>(),
        [
            app.world().resource::<ConsoleTheme>().output,
            Color::srgb(0.8, 0.6, 0.2),
            Color::srgb(0.9, 0.2, 0.3),
            Color::srgb(0.2, 0.4, 0.6),
            Color::srgb(0.2, 0.4, 0.6)
        ]
    );
}

/// Theme changes update colors and metrics while arrivals preserve section entities.
#[test]
fn sections_reuse_entities_update_theme_and_clear_without_stale_text() {
    let mut app = App::new();
    app.add_plugins(PromptlineUiPlugin);
    app.world_mut()
        .resource_mut::<Output>()
        .push(Severity::Info, "test", "first".into());
    app.world_mut()
        .resource_mut::<Output>()
        .push(Severity::Warn, "test", "second".into());
    app.update();
    let root = app
        .world_mut()
        .query::<(Entity, &ConsoleRole)>()
        .iter(app.world())
        .find_map(|(entity, role)| (*role == ConsoleRole::Output).then_some(entity))
        .unwrap();
    let section = app.world().get::<Children>(root).unwrap()[0];
    let original_color = app.world().get::<TextColor>(section).unwrap().0;
    app.update();
    assert_eq!(app.world().get::<Children>(root).unwrap()[0], section);
    {
        let mut theme = app.world_mut().resource_mut::<ConsoleTheme>();
        theme.background = Color::WHITE;
        theme.font.font_size = FontSize::Px(23.0);
    }
    app.update();
    assert_ne!(
        app.world().get::<TextColor>(section).unwrap().0,
        original_color
    );
    assert_eq!(
        app.world().get::<TextFont>(section).unwrap().font_size,
        FontSize::Px(23.0)
    );
    app.world_mut()
        .resource_mut::<Output>()
        .push(Severity::Error, "test", "third".into());
    app.update();
    assert_eq!(app.world().get::<Children>(root).unwrap()[0], section);
    assert_eq!(app.world().get::<TextSpan>(section).unwrap().0, "\nsecond");
    app.world_mut().resource_mut::<Output>().clear();
    app.update();
    assert!(app.world().get::<Text>(root).unwrap().0.is_empty());
    assert!(app.world().get_entity(section).is_err());
}
