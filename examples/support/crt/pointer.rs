//! Virtual pointer routing follows shader sampling and clamps active drags after leaving the glass.
use super::{
    mapping,
    scene::{
        MainCamera, SCREEN_CENTER, SCREEN_HEIGHT, SCREEN_WIDTH, Screen, ScreenTarget, TEXTURE_SIZE,
    },
    settings::Settings,
};
use bevy::asset::uuid::Uuid;
use bevy::camera::RenderTarget;
use bevy::input::ButtonState;
use bevy::picking::{
    backend::ray::RayMap,
    pointer::{Location, PointerAction, PointerId, PointerInput, PointerLocation},
};
use bevy::prelude::*;
use bevy::window::WindowEvent;
use bevy_promptline::input::ConsoleState;

/// This pointer identity is reserved for this independent example's one offscreen console.
pub(crate) const VIRTUAL_POINTER: PointerId =
    PointerId::Custom(Uuid::from_u128(0x42be_1910_1024_0640));
/// Virtual pointer state keeps a pressed selection alive until release without stale hover after a miss.
#[derive(Default)]
pub(crate) struct PointerState {
    /// Last mapped source-texture pixel position.
    last: Option<Vec2>,
    /// Active primary drag retains its clamp policy outside the physical glass.
    dragging: Option<PointerButton>,
}
/// Routes physical screen hits to source-image pixels through the exact shader sampling transform.
pub(crate) fn drive(
    mut state: Local<'_, PointerState>,
    mut raycast: MeshRayCast<'_, '_>,
    rays: Res<'_, RayMap>,
    screens: Query<'_, '_, Entity, With<Screen>>,
    cameras: Query<'_, '_, Entity, With<MainCamera>>,
    target: Option<Res<'_, ScreenTarget>>,
    settings: Res<'_, Settings>,
    mut events: MessageReader<'_, '_, WindowEvent>,
    mut inputs: MessageWriter<'_, PointerInput>,
    mut locations: Query<'_, '_, (&PointerId, &mut PointerLocation)>,
    mut console: ResMut<'_, ConsoleState>,
) {
    let Some(target) = target else {
        return;
    };
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    let render_target = RenderTarget::Image(target.image.clone().into())
        .normalize(None)
        .expect("image target normalizes without a window");
    let ray = rays.iter().find_map(|(id, ray)| {
        (id.camera == camera && id.pointer != VIRTUAL_POINTER).then_some(*ray)
    });
    let settings_cast = MeshRayCastSettings {
        visibility: RayCastVisibility::VisibleInView,
        filter: &|entity| screens.contains(entity),
        early_exit_test: &|_| false,
    };
    let hit = ray.and_then(|ray| {
        raycast
            .cast_ray(ray, &settings_cast)
            .iter()
            .find_map(|(_, hit)| hit.uv)
    });
    let valid_hit = hit.and_then(|uv| mapping::interaction_uv(uv, settings.effective_curvature()));
    let mapped = valid_hit
        .or_else(|| {
            if state.dragging.is_some() {
                ray.and_then(extended_uv)
                    .map(|uv| mapping::drag_uv(uv, settings.effective_curvature()))
            } else {
                None
            }
        })
        .map(|uv| uv * TEXTURE_SIZE);
    if let Some(position) = mapped {
        let delta = position - state.last.unwrap_or(position);
        inputs.write(PointerInput::new(
            VIRTUAL_POINTER,
            Location {
                target: render_target.clone(),
                position,
            },
            PointerAction::Move { delta },
        ));
        state.last = Some(position);
    } else if state.dragging.is_none() {
        cancel(&mut state, &render_target, &mut inputs, &mut locations);
    }
    for event in events.read() {
        match event {
            WindowEvent::MouseWheel(event) => {
                // A clamped drag outside the glass cannot acquire scroll ownership.
                let position = valid_hit.map(|uv| uv * TEXTURE_SIZE);
                if let Some(input) = wheel_input(position, &render_target, event) {
                    inputs.write(input);
                }
            }
            WindowEvent::MouseButtonInput(event) => {
                let button = match event.button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Right => PointerButton::Secondary,
                    MouseButton::Middle => PointerButton::Middle,
                    _ => continue,
                };
                if let Some(position) = state.last {
                    let action = match event.state {
                        ButtonState::Pressed => {
                            if button == PointerButton::Primary {
                                state.dragging = Some(button);
                                console.open();
                            }
                            PointerAction::Press(button)
                        }
                        ButtonState::Released => {
                            state.dragging = None;
                            PointerAction::Release(button)
                        }
                    };
                    inputs.write(PointerInput::new(
                        VIRTUAL_POINTER,
                        Location {
                            target: render_target.clone(),
                            position,
                        },
                        action,
                    ));
                    if event.state == ButtonState::Released && valid_hit.is_none() {
                        cancel(&mut state, &render_target, &mut inputs, &mut locations);
                    }
                }
            }
            WindowEvent::TouchInput(event) => {
                if let Some(position) = state.last {
                    let action = match event.phase {
                        bevy::input::touch::TouchPhase::Started => {
                            state.dragging = Some(PointerButton::Primary);
                            console.open();
                            PointerAction::Press(PointerButton::Primary)
                        }
                        bevy::input::touch::TouchPhase::Ended => {
                            state.dragging = None;
                            PointerAction::Release(PointerButton::Primary)
                        }
                        bevy::input::touch::TouchPhase::Canceled => {
                            state.dragging = None;
                            PointerAction::Cancel
                        }
                        bevy::input::touch::TouchPhase::Moved => continue,
                    };
                    inputs.write(PointerInput::new(
                        VIRTUAL_POINTER,
                        Location {
                            target: render_target.clone(),
                            position,
                        },
                        action,
                    ));
                }
            }
            WindowEvent::WindowFocused(event) if !event.focused => {
                cancel(&mut state, &render_target, &mut inputs, &mut locations);
                console.release_focus();
            }
            _ => {}
        }
    }
}
/// Preserves wheel units and phase while changing only the pointer and target.
fn wheel_input(
    position: Option<Vec2>,
    target: &bevy::camera::NormalizedRenderTarget,
    wheel: &bevy::input::mouse::MouseWheel,
) -> Option<PointerInput> {
    let position = position?;
    Some(PointerInput::new(
        VIRTUAL_POINTER,
        Location {
            target: target.clone(),
            position,
        },
        PointerAction::Scroll {
            unit: wheel.unit,
            x: wheel.x,
            y: wheel.y,
            phase: wheel.phase,
        },
    ))
}
/// A missed drag intersects the same screen plane, then clamps sampling to the source rectangle.
fn extended_uv(ray: Ray3d) -> Option<Vec2> {
    let z_direction = ray.direction.z;
    if z_direction.abs() < 0.000_001 {
        return None;
    }
    // Meter distance divided by dimensionless direction yields ray distance in meters.
    let distance = (SCREEN_CENTER.z - ray.origin.z) / z_direction;
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    let point = ray.get_point(distance) - SCREEN_CENTER;
    Some(Vec2::new(
        point.x / SCREEN_WIDTH + 0.5,
        0.5 - point.y / SCREEN_HEIGHT,
    ))
}
/// Cancel clears hover and pressed state instead of leaving a pointer at its last valid location.
fn cancel(
    state: &mut PointerState,
    target: &bevy::camera::NormalizedRenderTarget,
    inputs: &mut MessageWriter<'_, PointerInput>,
    locations: &mut Query<'_, '_, (&PointerId, &mut PointerLocation)>,
) {
    if let Some(position) = state.last.take() {
        inputs.write(PointerInput::new(
            VIRTUAL_POINTER,
            Location {
                target: target.clone(),
                position,
            },
            PointerAction::Cancel,
        ));
    }
    state.dragging = None;
    for (id, mut location) in locations {
        if *id == VIRTUAL_POINTER {
            location.location = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
    use bevy::input::touch::TouchPhase;

    /// Scrolling retains the physical wheel units at the mapped image location.
    #[test]
    fn wheel_routes_only_a_current_glass_hit() {
        let target = RenderTarget::Image(Handle::<Image>::default().into())
            .normalize(None)
            .expect("image target");
        for unit in [MouseScrollUnit::Line, MouseScrollUnit::Pixel] {
            for phase in [
                TouchPhase::Started,
                TouchPhase::Moved,
                TouchPhase::Ended,
                TouchPhase::Canceled,
            ] {
                let wheel = MouseWheel {
                    unit,
                    x: -2.0,
                    y: 3.5,
                    window: Entity::PLACEHOLDER,
                    phase,
                };
                assert!(wheel_input(None, &target, &wheel).is_none());
                let input = wheel_input(Some(Vec2::new(120.0, 240.0)), &target, &wheel)
                    .expect("current hit");
                assert_eq!(input.pointer_id, VIRTUAL_POINTER);
                assert_eq!(input.location.target, target);
                assert_eq!(input.location.position, Vec2::new(120.0, 240.0));
                assert!(matches!(input.action, PointerAction::Scroll {
                unit: actual, x, y, phase: actual_phase
            } if actual == unit && actual_phase == phase && x.to_bits() == (-2.0_f32).to_bits()
                && y.to_bits() == 3.5_f32.to_bits()));
            }
        }
    }
}
