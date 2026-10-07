//! App-thread draining does not install or replace the host's global
//! subscriber.
use super::{DisplayFilter, LogCapture, LogPresentation, LogStore};
use bevy::prelude::*;

/// Drains an explicitly installed capture queue into bounded app-thread log
/// retention. An absent capture resource leaves ordinary output working and
/// never attempts to retrofit a global subscriber.
#[derive(Debug, Default, Clone, Copy)]
pub struct PromptlineLogPlugin;
impl Plugin for PromptlineLogPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogStore>()
            .init_resource::<DisplayFilter>()
            .init_resource::<LogPresentation>()
            .add_systems(
                PostUpdate,
                drain_capture.in_set(crate::sets::PromptlineSystems::Output),
            );
    }
}
/// At most 64 records drain per frame while an absent subscriber remains
/// harmless.
fn drain_capture(capture: Option<Res<'_, LogCapture>>, mut store: ResMut<'_, LogStore>) {
    if let Some(capture) = capture {
        // An unchanged final identity means no captured record entered retention.
        let previous = store.records().next_back().map(|record| record.sequence);
        capture.drain_into(store.bypass_change_detection());
        let latest = store.records().next_back().map(|record| record.sequence);
        // Preserve external changes while marking only successful capture drains.
        if latest != previous {
            store.set_changed();
        }
    }
}
