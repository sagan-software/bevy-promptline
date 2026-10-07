//! Procedural CRT workstation with Bevy UI rendered into its actual screen texture.
#[path = "support/crt/mod.rs"]
mod crt;
/// Runs the standalone workstation rather than a mode inside another example.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    crt::run()
}
