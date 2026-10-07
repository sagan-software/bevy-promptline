//! Validated dimensionless screen controls are converted to GPU uniforms only at rendering boundaries.
use bevy::prelude::*;
use std::str::FromStr;

/// Curvature is dimensionless and constrained to the monotonic interval zero through 0.16.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Curvature(f32);
impl Curvature {
    /// Returns the validated coefficient for the shared CPU and WGSL sampling transform.
    pub(crate) const fn get(self) -> f32 {
        self.0
    }
}
impl Default for Curvature {
    fn default() -> Self {
        Self(0.08)
    }
}
impl FromStr for Curvature {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        bounded(value, 0.0, 0.16, "curvature").map(Self)
    }
}
/// Scanline strength is dimensionless and bounded to zero through 0.4 for readable text.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Scanlines(f32);
impl Scanlines {
    /// Returns the dimensionless strength at the GPU serialization boundary.
    pub(crate) const fn get(self) -> f32 {
        self.0
    }
}
impl Default for Scanlines {
    fn default() -> Self {
        Self(0.18)
    }
}
impl FromStr for Scanlines {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        bounded(value, 0.0, 0.4, "scanlines").map(Self)
    }
}
/// Brightness is a dimensionless multiplier bounded to 0.2 through two.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Brightness(f32);
impl Brightness {
    /// Returns the dimensionless multiplier at the GPU serialization boundary.
    pub(crate) const fn get(self) -> f32 {
        self.0
    }
}
impl Default for Brightness {
    fn default() -> Self {
        Self(1.0)
    }
}
impl FromStr for Brightness {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        bounded(value, 0.2, 2.0, "brightness").map(Self)
    }
}
/// Effects can be enabled or bypassed without creating a second rendering implementation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Effects {
    /// Apply the screen's bounded static effects.
    #[default]
    Enabled,
    /// Sample the original UI texture without screen effects.
    Disabled,
}
impl FromStr for Effects {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("on") {
            Ok(Self::Enabled)
        } else if value.eq_ignore_ascii_case("off") {
            Ok(Self::Disabled)
        } else {
            Err("effect expects on or off".into())
        }
    }
}
/// Phosphor tint is a closed visual vocabulary rather than an arbitrary wire color.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) enum Tint {
    /// Readable green phosphor is the default.
    #[default]
    Green,
    /// Amber phosphor offers a warm comparison.
    Amber,
    /// Neutral white preserves the original UI color balance.
    White,
}
impl FromStr for Tint {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.eq_ignore_ascii_case("green") {
            Ok(Self::Green)
        } else if value.eq_ignore_ascii_case("amber") {
            Ok(Self::Amber)
        } else if value.eq_ignore_ascii_case("white") {
            Ok(Self::White)
        } else {
            Err("tint expects green, amber, or white".into())
        }
    }
}
impl Tint {
    /// Linear RGBA coefficients multiply the sampled console texture.
    pub(crate) const fn uniform(self) -> Vec4 {
        match self {
            Self::Green => Vec4::new(0.5, 1.0, 0.62, 1.0),
            Self::Amber => Vec4::new(1.0, 0.66, 0.25, 1.0),
            Self::White => Vec4::ONE,
        }
    }
}
/// One settings authority drives both the material uniforms and physical pointer mapping.
#[derive(Debug, Default, Clone, Resource)]
pub(crate) struct Settings {
    /// Validated radial sampling coefficient.
    pub(crate) curvature: Curvature,
    /// Validated static scanline strength.
    pub(crate) scanlines: Scanlines,
    /// Validated sampled-light multiplier.
    pub(crate) brightness: Brightness,
    /// Selected phosphor tint.
    pub(crate) tint: Tint,
    /// Screen effect enablement keeps flat comparison explicit.
    pub(crate) effects: Effects,
}
impl Settings {
    /// Bypass uses zero curvature in both CPU and shader instead of desynchronizing them.
    pub(crate) const fn effective_curvature(&self) -> f32 {
        match self.effects {
            Effects::Enabled => self.curvature.get(),
            Effects::Disabled => 0.0,
        }
    }
    /// Raw coefficients cross only the GPU serialization boundary as one aligned vector.
    pub(crate) fn uniform(&self) -> Vec4 {
        Vec4::new(
            self.brightness.get(),
            self.scanlines.get(),
            self.effective_curvature(),
            if self.effects == Effects::Enabled {
                1.0
            } else {
                0.0
            },
        )
    }
}
/// Parses finite dimensionless values before they reach settings or GPU uniforms.
fn bounded(value: &str, minimum: f32, maximum: f32, name: &str) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{name} expects a number"))?;
    if !parsed.is_finite() || !(minimum..=maximum).contains(&parsed) {
        return Err(format!(
            "{name} must be finite and between {minimum} and {maximum}"
        ));
    }
    Ok(parsed)
}

/// Each accepted setting action contains a validated payload for exactly one control.
#[derive(Debug)]
enum SettingCommand {
    /// Replace the bounded radial coefficient.
    Curvature(Curvature),
    /// Replace the bounded static scanline strength.
    Scanlines(Scanlines),
    /// Replace the bounded dimensionless brightness multiplier.
    Brightness(Brightness),
    /// Select a closed phosphor tint.
    Tint(Tint),
    /// Enable or bypass screen effects.
    Effects(Effects),
    /// Restore stable default effects without moving noise or flicker.
    Reset,
    /// Remove distortion, scanlines, and RGB separation through the bypass mode.
    Reduced,
}
/// Parses closed control names while preserving the actual command draft and value spelling.
fn parse(
    arguments: &[String],
) -> Result<bevy_promptline::input::ParseDecision<SettingCommand>, String> {
    use bevy_promptline::input::ParseDecision;
    let command = match arguments {
        [action] if action.eq_ignore_ascii_case("reset") => SettingCommand::Reset,
        [action] if action.eq_ignore_ascii_case("reduced") => SettingCommand::Reduced,
        [action, value] => match action.to_ascii_lowercase().as_str() {
            "curvature" => SettingCommand::Curvature(value.parse()?),
            "scanlines" => SettingCommand::Scanlines(value.parse()?),
            "brightness" => SettingCommand::Brightness(value.parse()?),
            "tint" => SettingCommand::Tint(value.parse()?),
            "effect" => SettingCommand::Effects(value.parse()?),
            _ => return Err("CRT SCANLINES | CURVATURE | BRIGHTNESS | TINT | EFFECT | RESET | REDUCED".into()),
        },
        _ => return Ok(ParseDecision::Information("CRT CURVATURE 0..0.16 | SCANLINES 0..0.4 | BRIGHTNESS 0.2..2\nCRT TINT GREEN|AMBER|WHITE | EFFECT ON|OFF | RESET | REDUCED".into())),
    };
    Ok(ParseDecision::Run(command))
}
/// Applies only validated controls to the one settings authority used by CPU and GPU.
fn execute(
    In(command): In<SettingCommand>,
    mut settings: ResMut<'_, Settings>,
    mut output: ResMut<'_, bevy_promptline::Output>,
) -> bevy_promptline::CommandOutcome {
    match command {
        SettingCommand::Curvature(value) => settings.curvature = value,
        SettingCommand::Scanlines(value) => settings.scanlines = value,
        SettingCommand::Brightness(value) => settings.brightness = value,
        SettingCommand::Tint(value) => settings.tint = value,
        SettingCommand::Effects(value) => settings.effects = value,
        SettingCommand::Reset => *settings = default(),
        SettingCommand::Reduced => settings.effects = Effects::Disabled,
    }
    output.write("CRT settings updated");
    bevy_promptline::CommandOutcome::Success
}
/// Static control choices are metadata and never execute their numeric validators while typing.
fn complete(
    InRef(request): InRef<'_, bevy_promptline::input::CompletionRequest>,
) -> Vec<bevy_promptline::input::Candidate> {
    let values: &[&str] = if request.position() == 1 {
        &[
            "CURVATURE",
            "SCANLINES",
            "BRIGHTNESS",
            "TINT",
            "EFFECT",
            "RESET",
            "REDUCED",
        ]
    } else if request.position() == 2 {
        match request
            .tokens()
            .get(1)
            .map(|token| token.value.to_ascii_uppercase())
            .as_deref()
        {
            Some("TINT") => &["GREEN", "AMBER", "WHITE"],
            Some("EFFECT") => &["ON", "OFF"],
            _ => &[],
        }
    } else {
        &[]
    };
    values
        .iter()
        .filter_map(|value| {
            let prefix = request.prefix();
            if !value.get(..prefix.len())?.eq_ignore_ascii_case(prefix) {
                return None;
            }
            let suffix = value.get(prefix.len()..)?;
            Some(bevy_promptline::input::Candidate {
                value: format!("{prefix}{suffix}"),
                description: "CRT control".into(),
            })
        })
        .collect()
}
/// Registers shader controls through the same typed parsing and execution seam as DOS commands.
pub(crate) fn install(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    use bevy_promptline::PromptlineAppExt;
    app.init_resource::<Settings>();
    let metadata = bevy_promptline::CommandMetadata::new(
        "CRT",
        "Screen-only effects; strengths are dimensionless",
    )?;
    app.register_builder(
        bevy_promptline::input::CommandBuilder::new(metadata, parse),
        execute,
    )?;
    app.register_completion_system("CRT", complete)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setting_ranges_reject_nonfinite_and_outside_values_before_uniforms() {
        for value in ["NaN", "inf", "-0.001", "0.161", "word"] {
            assert!(value.parse::<Curvature>().is_err());
        }
        for value in ["0", "0.16"] {
            assert!(value.parse::<Curvature>().is_ok());
        }
        for value in ["-0.001", "0.401", "NaN"] {
            assert!(value.parse::<Scanlines>().is_err());
        }
        for value in ["0", "0.4"] {
            assert!(value.parse::<Scanlines>().is_ok());
        }
        for value in ["0.19", "2.01", "NaN"] {
            assert!(value.parse::<Brightness>().is_err());
        }
        for value in ["0.2", "2"] {
            assert!(value.parse::<Brightness>().is_ok());
        }
        let settings = Settings {
            effects: Effects::Disabled,
            ..default()
        };
        assert!(settings.effective_curvature().abs() < f32::EPSILON);
        assert!(settings.uniform().w.abs() < f32::EPSILON);
    }
}
