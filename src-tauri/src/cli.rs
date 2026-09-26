//! Command-line handoff:
//!
//! - `floppy import [--os dos|mac-classic|amiga] <path>`: an app.
//! - `floppy open [--os dos] <file>`: a document, to open in an app that
//!   made it (documents.rs). DOS only so far.
//!
//! This is how another program (a catalog app like Diskette, a script)
//! hands Floppy an app or a file: start Floppy with these arguments, and
//! it imports the path into the library and opens with it selected. Only
//! arguments cross the process boundary. Floppy never reads another
//! program's data.

use std::path::PathBuf;

use crate::library::GuestOs;

#[derive(Debug, PartialEq)]
pub enum Cli {
    /// Plain launch (includes arguments macOS itself adds, like `-psn_…`).
    None,
    Import { os: GuestOs, path: PathBuf },
    Open { os: GuestOs, path: PathBuf },
    Error(String),
}

const USAGE: &str = "Usage: floppy import [--os dos|mac-classic|amiga] <path>, or floppy open [--os dos] <file>";

/// Parses `args` without the program name.
pub fn parse(args: &[String]) -> Cli {
    let mut it = args.iter();
    let open = match it.next().map(String::as_str) {
        Some("import") => false,
        Some("open") => true,
        _ => return Cli::None,
    };
    let mut os = GuestOs::Dos;
    let mut path = None;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--os" => match it.next().and_then(|s| GuestOs::parse(s)) {
                Some(o) => os = o,
                None => return Cli::Error(format!("Unknown or missing guest OS. Supported: dos, mac-classic, amiga. {USAGE}")),
            },
            _ if path.is_none() => path = Some(PathBuf::from(arg)),
            _ => return Cli::Error(format!("Unexpected argument: {arg}. {USAGE}")),
        }
    }
    match path {
        Some(path) if open => Cli::Open { os, path },
        Some(path) => Cli::Import { os, path },
        None => Cli::Error(format!("Missing the path. {USAGE}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Cli {
        parse(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_import() {
        assert_eq!(p(&["import", "--os", "dos", "/v/WP51"]), Cli::Import { os: GuestOs::Dos, path: "/v/WP51".into() });
        assert_eq!(p(&["import", "/v/PKZIP.EXE"]), Cli::Import { os: GuestOs::Dos, path: "/v/PKZIP.EXE".into() });
        assert_eq!(p(&["import", "/v/A", "--os", "DOS"]), Cli::Import { os: GuestOs::Dos, path: "/v/A".into() });
        assert_eq!(
            p(&["import", "--os", "mac-classic", "/v/MacWrite II"]),
            Cli::Import { os: GuestOs::MacClassic, path: "/v/MacWrite II".into() }
        );
        assert_eq!(p(&["import", "--os", "amiga", "/v/PT.adf"]), Cli::Import { os: GuestOs::Amiga, path: "/v/PT.adf".into() });
    }

    #[test]
    fn parses_open() {
        assert_eq!(p(&["open", "/v/LETTER.WP5"]), Cli::Open { os: GuestOs::Dos, path: "/v/LETTER.WP5".into() });
        assert_eq!(p(&["open", "--os", "dos", "/v/B.WK1"]), Cli::Open { os: GuestOs::Dos, path: "/v/B.WK1".into() });
        assert!(matches!(p(&["open"]), Cli::Error(_)));
    }

    #[test]
    fn ignores_plain_launch_and_rejects_bad_import() {
        assert_eq!(p(&[]), Cli::None);
        assert_eq!(p(&["-psn_0_12345"]), Cli::None);
        assert!(matches!(p(&["import"]), Cli::Error(_)));
        assert!(matches!(p(&["import", "--os", "c64", "/x"]), Cli::Error(_)));
        assert!(matches!(p(&["import", "--os"]), Cli::Error(_)));
        assert!(matches!(p(&["import", "/a", "/b"]), Cli::Error(_)));
    }
}
