//! Schedule boundaries shared by runtime dispatch and replaceable
//! presentations.
use bevy::prelude::SystemSet;

/// Orders console input, analysis, dispatch, output processing, and
/// presentation. Applications use these sets to coordinate cooperating game
/// systems with the console's frame work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum PromptlineSystems {
    /// Processes console visibility requests and keyboard ownership before
    /// cooperating game input. Applications order their keyboard systems after
    /// this set and apply the public focus gate.
    Input,
    /// Reads the applied editor value and derives metadata without invoking a
    /// parser. This set follows Bevy's text edit application so submission uses
    /// actual edited input.
    Analysis,
    /// Validates and executes queued commands in submission order on the app
    /// thread. A registered handler owns execution after acceptance and cannot
    /// run for rejected input.
    Dispatch,
    /// Consumes runtime notifications and updates console output after command
    /// dispatch. Applications can coordinate their output consumers here
    /// without becoming additional command execution owners.
    Output,
    /// Updates console content and default presentation from the runtime and
    /// editor state. Applications order custom layout or animation here while
    /// using the public role components.
    Presentation,
}
