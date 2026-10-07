//! Display UV and source texture UV use the same documented sampling transform as WGSL.
use bevy::prelude::*;

/// Maps displayed UV to source UV; u increases rightward and v increases downward from the top.
/// Curvature is dimensionless. For nonnegative curvature, radial and tangential derivatives remain positive.
pub(crate) fn sample_uv(display: Vec2, curvature: f32) -> Vec2 {
    // Normalized coordinates shift from [0,1] to [-1,1] around the glass center.
    let centered = display * 2.0 - Vec2::ONE;
    // Keep the scalar stages explicit to follow the WGSL sampling expression.
    let bend = curvature * centered.length_squared();
    let scale = 1.0 + bend;
    Vec2::splat(0.5) + centered * 0.5 * scale
}

/// Accepts only visible glass whose sampled source coordinate remains inside the framebuffer.
pub(crate) fn interaction_uv(display: Vec2, curvature: f32) -> Option<Vec2> {
    if !display.is_finite() || display.min_element() < 0.0 || display.max_element() > 1.0 {
        return None;
    }
    // The normalized corner radius and clamp match on_glass in promptline_crt.wgsl exactly.
    let radius = 0.045;
    let nearest = display.clamp(Vec2::splat(radius), Vec2::splat(1.0 - radius));
    if display.distance(nearest) > radius {
        return None;
    }
    let source = sample_uv(display, curvature);
    if source.min_element() < 0.0 || source.max_element() > 1.0 {
        return None;
    }
    Some(source)
}

/// Active selection drags clamp to the source rectangle until release, including a glass miss.
pub(crate) fn drag_uv(display: Vec2, curvature: f32) -> Vec2 {
    sample_uv(display.clamp(Vec2::ZERO, Vec2::ONE), curvature).clamp(Vec2::ZERO, Vec2::ONE)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordered_corners_mask_center_stays_fixed_and_extremes_are_one_to_one() {
        for corner in [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y] {
            assert!(interaction_uv(corner, 0.0).is_none());
            assert!(interaction_uv(corner, 0.16).is_none());
        }
        assert_eq!(
            interaction_uv(Vec2::splat(0.5), 0.16),
            Some(Vec2::splat(0.5))
        );
        let left = sample_uv(Vec2::new(0.25, 0.5), 0.16);
        let center = sample_uv(Vec2::splat(0.5), 0.16);
        let right = sample_uv(Vec2::new(0.75, 0.5), 0.16);
        assert!(left.x < center.x && center.x < right.x);
        assert!((left.x - 0.24).abs() * 1024.0 < 1.0);
        assert!((right.x - 0.76).abs() * 1024.0 < 1.0);
        assert!(interaction_uv(Vec2::new(-0.1, 0.5), 0.0).is_none());
        assert!(interaction_uv(Vec2::new(0.0, 0.5), 0.16).is_none());
        assert_eq!(drag_uv(Vec2::new(-0.2, 1.2), 0.16), Vec2::new(0.0, 1.0));
    }
}
