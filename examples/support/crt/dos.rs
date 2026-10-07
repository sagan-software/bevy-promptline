//! Read-only in-memory DOS subset uses ordinary command dispatch and explicit ASCII name matching.
use bevy::prelude::*;
use bevy_promptline::input::{
    Candidate, CommandBuilder, CompletionRequest, NameMatching, ParseDecision,
};
use bevy_promptline::{CommandMetadata, CommandOutcome, Output, PromptlineAppExt};
use std::fmt::Write as _;
use std::str::FromStr;

/// Current virtual directory is a closed vocabulary, so no OS path can enter this model.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Directory {
    /// Virtual drive root contains its guide and the DOCS child directory.
    #[default]
    Root,
    /// Read-only child directory contains command and scene descriptions.
    Docs,
}
/// A simple filename preserves argument spelling while validating its accepted ASCII grammar.
#[derive(Debug)]
struct Filename(String);
impl FromStr for Filename {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.is_empty()
            || value.len() > 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err("Names use 1..32 ASCII letters, digits, underscore, hyphen, or dot".into());
        }
        Ok(Self(value.to_owned()))
    }
}
impl AsRef<str> for Filename {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
/// Virtual directory state is independent of the authoritative command draft and shader settings.
#[derive(Debug, Default, Resource)]
pub(crate) struct VirtualFs {
    /// Current directory is always one of the two read-only virtual locations.
    cwd: Directory,
}
/// Read-only root guide is embedded data rather than an operating-system file.
const ROOT_FILES: &[(&str, &str)] = &[(
    "README.TXT",
    "Bevy Promptline\nHELP lists commands. CD DOCS enters the read-only guide directory.\nTYPE reads a file. CD .. returns to the root.\nNames compare without ASCII case sensitivity. Real filesystem access is unavailable.",
)];
/// Read-only command and scene descriptions belong to the virtual DOCS directory.
const DOC_FILES: &[(&str, &str)] = &[
    (
        "COMMANDS.TXT",
        "HELP DIR CD TYPE CLS ECHO VER\nCD DOCS | CD ..\nTYPE README.TXT\nECHO \"quoted text\"\nCRT CURVATURE 0..0.16\nCRT SCANLINES 0..0.4\nCRT TINT GREEN|AMBER|WHITE\nCRT EFFECT ON|OFF\nCRT RESET",
    ),
    (
        "SCENE.TXT",
        "The desk, monitor, case, and keyboard are procedural Bevy meshes.\nThe screen samples an actual offscreen Bevy UI texture.\nPointer input uses the shader's same sampling transform.\nEscape releases focus before the desk view returns.",
    ),
];
impl VirtualFs {
    /// Borrows read-only file entries without collecting or consulting the operating system.
    const fn files(&self) -> &'static [(&'static str, &'static str)] {
        match self.cwd {
            Directory::Root => ROOT_FILES,
            Directory::Docs => DOC_FILES,
        }
    }
    /// Emits the virtual prompt without changing stored filenames or user arguments.
    pub(crate) const fn prompt(&self) -> &'static str {
        match self.cwd {
            Directory::Root => "C:\\>",
            Directory::Docs => "C:\\DOCS>",
        }
    }
}
/// Distinct DOS actions contain only the arguments required by their operation.
#[derive(Debug)]
enum DosCommand {
    /// Print the supported command subset.
    Help,
    /// List current read-only directory entries.
    Dir,
    /// Resolve a simple child name or parent marker against the current virtual directory.
    Cd(Filename),
    /// Read one simple file name from the current virtual directory.
    Type(Filename),
    /// Clear retained console output without changing filesystem or input state.
    Clear,
    /// Print decoded argument text without interpreting command chaining or escapes again.
    Echo(String),
    /// Print the plugin and example version.
    Version,
}
/// Registration selects one operation without giving constructors access to ECS state.
#[derive(Debug, Clone, Copy)]
enum Kind {
    /// Print the command guide.
    Help,
    /// List virtual entries.
    Dir,
    /// Change virtual directory.
    Cd,
    /// Read a virtual file.
    Type,
    /// Clear direct output.
    Clear,
    /// Print decoded text.
    Echo,
    /// Print version information.
    Version,
}
/// A missing argument yields useful information without executing a game handler.
fn parse(kind: Kind, arguments: &[String]) -> Result<ParseDecision<DosCommand>, String> {
    let command = match kind {
        Kind::Help if arguments.is_empty() => DosCommand::Help,
        Kind::Dir if arguments.is_empty() => DosCommand::Dir,
        Kind::Clear if arguments.is_empty() => DosCommand::Clear,
        Kind::Version if arguments.is_empty() => DosCommand::Version,
        Kind::Echo => DosCommand::Echo(arguments.join(" ")),
        Kind::Cd => match arguments {
            [name] => DosCommand::Cd(name.parse()?),
            [] => return Ok(ParseDecision::Information("CD DOCS | CD .. | CD .".into())),
            _ => return Err("CD expects one simple directory name".into()),
        },
        Kind::Type => match arguments {
            [name] => DosCommand::Type(name.parse()?),
            [] => return Ok(ParseDecision::Information("TYPE <simple filename>".into())),
            _ => return Err("TYPE expects one simple filename".into()),
        },
        _ => return Err("This command has no arguments".into()),
    };
    Ok(ParseDecision::Run(command))
}
/// Handler effects stay on the app thread after parsing and permission checks succeed.
fn execute(
    In(command): In<DosCommand>,
    mut fs: ResMut<'_, VirtualFs>,
    mut output: ResMut<'_, Output>,
) -> CommandOutcome {
    match command {
        DosCommand::Help => output.write("HELP DIR CD TYPE CLS ECHO VER\nCD DOCS | CD .. | CD .\nTYPE <filename>\nCRT SCANLINES 0..0.4 | CURVATURE 0..0.16 | TINT GREEN|AMBER|WHITE\nCRT EFFECT ON|OFF | RESET\nClick the glass or Edit screen to focus. Escape returns to the desk."),
        DosCommand::Dir => {
            let mut listing = String::new();
            if fs.cwd == Directory::Root { listing.push_str("DOCS         <DIR>\n"); }
            for (name, contents) in fs.files() { let bytes = contents.len(); let _ = writeln!(listing, "{name:<12} {bytes:>5} bytes"); }
            output.write(listing);
        }
        DosCommand::Cd(name) => {
            let name = name.as_ref();
            if name == ".." { fs.cwd = Directory::Root; }
            else if name == "." {} else if fs.cwd == Directory::Root && name.eq_ignore_ascii_case("DOCS") { fs.cwd = Directory::Docs; }
            else { output.write("Directory not found"); return CommandOutcome::Failure("directory not found".into()); }
            output.write(fs.prompt());
        }
        DosCommand::Type(name) => {
            if let Some((_, contents)) = fs.files().iter().find(|(stored, _)| stored.eq_ignore_ascii_case(name.as_ref())) { output.write(*contents); }
            else { output.write("File not found"); return CommandOutcome::Failure("file not found".into()); }
        }
        DosCommand::Clear => output.clear(),
        DosCommand::Echo(text) => output.write(text),
        DosCommand::Version => output.write(concat!("Bevy Promptline ", env!("CARGO_PKG_VERSION"), " / Bevy 0.19.1 / virtual DOS subset")),
    }
    CommandOutcome::Success
}
/// Completion preserves the already typed prefix while comparing virtual names without ASCII case.
fn complete(
    InRef(request): InRef<'_, CompletionRequest>,
    fs: Res<'_, VirtualFs>,
) -> Vec<Candidate> {
    if request.position() != 1 {
        return Vec::new();
    }
    let command = request
        .tokens()
        .first()
        .map_or("", |token| token.value.as_str());
    let names: Vec<&str> = if command.eq_ignore_ascii_case("CD") {
        if fs.cwd == Directory::Root {
            vec!["DOCS", "..", "."]
        } else {
            vec!["..", "."]
        }
    } else {
        fs.files().iter().map(|(name, _)| *name).collect()
    };
    names
        .into_iter()
        .filter_map(|name| {
            let prefix = request.prefix();
            let head = name.get(..prefix.len())?;
            if !head.eq_ignore_ascii_case(prefix) {
                return None;
            }
            let suffix = name.get(prefix.len()..)?;
            Some(Candidate {
                value: format!("{prefix}{suffix}"),
                description: "Read-only virtual entry".into(),
            })
        })
        .collect()
}
/// Installs the local case profile before registering the example's explicit command subset.
pub(crate) fn install(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    app.init_resource::<VirtualFs>();
    app.world_mut()
        .resource_mut::<bevy_promptline::CommandRegistry>()
        .set_name_matching(NameMatching::AsciiInsensitive)?;
    for (name, kind, help) in [
        ("HELP", Kind::Help, "List DOS commands"),
        ("DIR", Kind::Dir, "List read-only virtual entries"),
        ("CD", Kind::Cd, "Change directory: CD DOCS | CD .."),
        ("TYPE", Kind::Type, "Read a file: TYPE README.TXT"),
        ("CLS", Kind::Clear, "Clear console output"),
        ("ECHO", Kind::Echo, "Print decoded text"),
        ("VER", Kind::Version, "Print version"),
    ] {
        let metadata = CommandMetadata::new(name, help)?;
        app.register_builder(
            CommandBuilder::new(metadata, move |arguments| parse(kind, arguments)),
            execute,
        )?;
    }
    app.register_completion_system("CD", complete)?;
    app.register_completion_system("TYPE", complete)?;
    app.world_mut()
        .resource_mut::<Output>()
        .write("Bevy Promptline / virtual C: drive\nHELP lists commands. DIR lists entries.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn virtual_commands_preserve_arguments_and_use_case_insensitive_relative_names() {
        let mut app = App::new();
        app.add_plugins(bevy_promptline::PromptlineRuntimePlugin);
        install(&mut app).unwrap();
        for input in [
            "eChO 'Mixed Case text'",
            "cD docs",
            "TyPe commands.txt",
            "CD ..",
            "CD ..",
        ] {
            app.world_mut()
                .write_message(bevy_promptline::SubmitCommand::new(input));
            app.update();
        }
        let texts: Vec<_> = app.world().resource::<Output>().texts().collect();
        assert!(texts.contains(&"Mixed Case text"));
        assert!(texts.iter().any(|text| text.contains("CRT CURVATURE")));
        assert_eq!(app.world().resource::<VirtualFs>().prompt(), "C:\\>");
        let analysis = bevy_promptline::input::analyze(app.world(), "type rea", 8).unwrap();
        assert_eq!(analysis.candidates()[0].value, "reaDME.TXT");
        app.world_mut()
            .write_message(bevy_promptline::SubmitCommand::new("CLS"));
        app.update();
        assert_eq!(app.world().resource::<Output>().texts().count(), 0);
    }
    #[test]
    fn simple_name_profile_rejects_paths_and_missing_type_prints_help_without_mutation() {
        for invalid in ["", "C:\\DOCS", "../DOCS", "with space", "é"] {
            assert!(invalid.parse::<Filename>().is_err());
        }
        let mut app = App::new();
        app.add_plugins(bevy_promptline::PromptlineRuntimePlugin);
        install(&mut app).unwrap();
        app.world_mut()
            .write_message(bevy_promptline::SubmitCommand::new("TYPE"));
        app.update();
        assert!(
            app.world()
                .resource::<Output>()
                .texts()
                .any(|text| text == "TYPE <simple filename>")
        );
        assert_eq!(app.world().resource::<VirtualFs>().prompt(), "C:\\>");
    }
}
