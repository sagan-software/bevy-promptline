//! Plain parser registration through the existing typed execution boundary.
use bevy::prelude::*;
use bevy_promptline::input::{CommandBuilder, ParseDecision};
use bevy_promptline::{
    CommandMetadata, CommandOutcome, Output, PromptlineAppExt, PromptlineRuntimePlugin,
    SubmitCommand,
};
use std::num::NonZeroU32;

/// Positive increment parsed without implementing the command metadata trait.
#[derive(Debug)]
struct Increment(NonZeroU32);
/// Accumulated test increments provide an observable handler side effect.
#[derive(Debug, Default, Resource)]
struct Counter(u32);

#[test]
fn plain_builder_delivers_typed_values_and_information_without_dispatching_help() {
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin)
        .init_resource::<Counter>();
    let mut metadata = CommandMetadata::new("count", "Increment a counter").unwrap();
    metadata.aliases.push("inc".parse().unwrap());
    let builder = CommandBuilder::new(metadata, |arguments: &[String]| match arguments {
        [argument] if argument == "--help" => {
            Ok(ParseDecision::Information("count <positive amount>".into()))
        }
        [argument] => argument
            .parse::<NonZeroU32>()
            .map(Increment)
            .map(ParseDecision::Run)
            .map_err(|error| error.to_string()),
        _ => Err("one positive amount is required".into()),
    });
    assert!(!format!("{builder:?}").contains("closure"));
    app.register_builder(
        builder,
        |In(increment): In<Increment>, mut counter: ResMut<'_, Counter>| {
            counter.0 += increment.0.get();
            CommandOutcome::Success
        },
    )
    .unwrap();
    for input in ["inc 9", "count --help", "count 0", "count"] {
        app.world_mut().write_message(SubmitCommand::new(input));
    }
    app.update();
    assert_eq!(app.world().resource::<Counter>().0, 9);
    assert!(
        app.world()
            .resource::<Output>()
            .texts()
            .any(|text| text == "count <positive amount>")
    );
}

#[test]
fn builder_registration_preserves_collision_atomicity_and_authorizes_before_parsing() {
    use bevy_promptline::{CommandRegistry, RegistrationError};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let parses = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&parses);
    let mut app = App::new();
    app.add_plugins(PromptlineRuntimePlugin)
        .init_resource::<Counter>();
    let builder = CommandBuilder::new(
        CommandMetadata::new("count", "Count calls").unwrap(),
        move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok(ParseDecision::Run(Increment(NonZeroU32::new(1).unwrap())))
        },
    );
    assert_eq!(parses.load(Ordering::Relaxed), 0);
    app.register_builder(
        builder,
        |In(increment): In<Increment>, mut counter: ResMut<'_, Counter>| {
            counter.0 += increment.0.get();
            CommandOutcome::Success
        },
    )
    .unwrap();
    let mut colliding = CommandMetadata::new("other", "Colliding alias").unwrap();
    colliding.aliases.push("count".parse().unwrap());
    let before = app.world().entities().len();
    let rejected = CommandBuilder::<Increment>::new(colliding, |_| panic!("rejected parser ran"));
    assert!(
        matches!(app.register_builder(rejected, |_: In<Increment>| CommandOutcome::Success),
        Err(RegistrationError::Collision(name)) if name == "count")
    );
    assert_eq!(app.world().entities().len(), before);
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(|_, _, _| false);
    app.world_mut().write_message(SubmitCommand::new("count"));
    app.update();
    assert_eq!(parses.load(Ordering::Relaxed), 0);
    assert_eq!(app.world().resource::<Counter>().0, 0);
    app.world_mut()
        .resource_mut::<CommandRegistry>()
        .set_policy(|_, _, _| true);
    app.world_mut().write_message(SubmitCommand::new("count"));
    app.update();
    assert_eq!(parses.load(Ordering::Relaxed), 1);
    assert_eq!(app.world().resource::<Counter>().0, 1);
}
