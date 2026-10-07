//! Browser ownership signals use DOM CustomEvent without carrying draft text or
//! command arguments.
use bevy::prelude::*;
use wasm_bindgen::JsValue;

/// Mirrors editing ownership as one Boolean detail value after console focus
/// requests apply.
/// DOM Living Standard, 2026-10-05: https://dom.spec.whatwg.org/#interface-customevent
pub(super) fn ownership(
    state: Res<'_, super::ConsoleState>,
    mut previous: Local<'_, Option<bool>>,
) {
    let is_editing = state.is_editing();
    if *previous == Some(is_editing) {
        return;
    }
    let Some(window) = web_sys::window() else {
        return;
    };
    let options = web_sys::CustomEventInit::new();
    options.set_detail(&JsValue::from_bool(is_editing));
    if let Ok(event) =
        web_sys::CustomEvent::new_with_event_init_dict("bevy-promptline:editing", &options)
    {
        if window.dispatch_event(&event).is_ok() {
            *previous = Some(is_editing);
        }
    }
}
