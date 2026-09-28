//! Dropped files: what each one is, and where it goes. Floppy judges a
//! dropped file, folder or zip by its contents first, then its name, then
//! the tab it was dropped on, and lists every place it could go with a
//! score:
//!
//! - 3, its contents say so: a DOS program's `MZ` header, an Amiga hunk
//!   executable, an AmigaDOS floppy, an HFS volume, a Mac `APPL`, a
//!   ROM or startup disk setup recognizes, an IFF or Mac type an app in
//!   `HANDLERS` opens;
//! - 2, its name says so: a `.com`, a disk image extension, an extension
//!   a known app opens; or it was dropped on that guest's tab;
//! - 1, it could go there, nothing says it does.
//!
//! One option scoring highest, at 2 or more, is a clear winner and is
//! used. Otherwise the user decides in a dialog, unless a choice for the
//! same kind of file ([`Signature`]: file, folder or zip, extension, and
//! what its contents look like) is already known: the user's own
//! remembered answer, then Floppy's rules (`docs/file-handling.md`'s
//! "Drop choices", merged from findings and read when Floppy is built),
//! then what other users chose (learned from findings dropped on
//! Floppy, learned.rs).
//!
//! Every answer is kept in `library/drop-choices.json` and goes out with
//! the next Export Findings (findings.rs), which is how Floppy's rules
//! grow. A signature never holds a file or folder name.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::cd::{self, Slot};
use crate::handlers;
use crate::learned;
use crate::library::{self, guest_label, GuestOs, Library};
use crate::{amiga, dos, mac};

/// Where a dropped item can go.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Dest {
    /// An app in the guest's library.
    App,
    /// A document in the guest's documents folder.
    Document,
    /// One of the guest's setup files (cd.rs).
    Setup,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub to: Dest,
    pub os: GuestOs,
}

impl Choice {
    pub fn label(self) -> String {
        let what = match self.to {
            Dest::App => "App",
            Dest::Document => "Document",
            Dest::Setup => "Setup file",
        };
        format!("{what}, {}", guest_label(self.os))
    }

    /// The other way round, as `docs/file-handling.md` writes it.
    pub fn parse(label: &str) -> Option<Choice> {
        let (what, guest) = label.split_once(',')?;
        let to = match what.trim() {
            "App" => Dest::App,
            "Document" => Dest::Document,
            "Setup file" => Dest::Setup,
            _ => return None,
        };
        Some(Choice { to, os: handlers::guest_from_label(guest.trim())? })
    }
}

/// What kind of item was dropped, without its name: what Floppy's rules
/// and the findings are keyed on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    /// `file`, `folder` or `zip`.
    pub kind: String,
    /// Lowercase, no dot; empty for none (and for folders).
    pub ext: String,
    /// What its contents look like: `mz`, `hunk`, `adf`, `hfs`,
    /// `iff:ILBM`, `mac-type:TEXT`, `macbinary:APPL`, `setup:kickstart`,
    /// `text`, `binary`, `empty`; for folders and zips, which guests'
    /// programs they hold (`programs:dos+mac`, `programs:none`) and any
    /// setup files (`+setup`).
    pub content: String,
}

impl Signature {
    /// Whether a signature from someone else's findings is well formed:
    /// short, printable, and without the table's `|`.
    pub fn is_valid(&self) -> bool {
        let ok = |s: &str, max: usize| s.len() <= max && s.bytes().all(|b| b.is_ascii_graphic() && b != b'|' && b != b'`');
        matches!(self.kind.as_str(), "file" | "folder" | "zip") && ok(&self.ext, 12) && ok(&self.content, 48)
    }

    /// For the dialog's "the same for other …" line.
    pub fn describe(&self) -> String {
        match self.kind.as_str() {
            "folder" => "folders like this".into(),
            "zip" => "zips like this".into(),
            _ if self.ext.is_empty() => "files like this with no extension".into(),
            _ => format!(".{} files like this", self.ext),
        }
    }
}

/// One place a dropped item could go.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DropOption {
    pub choice: Choice,
    /// "Document, DOS".
    pub label: String,
    /// 1 to 3 (see the module doc).
    pub score: u8,
    /// Why, in a sentence.
    pub why: String,
}

/// What Floppy made of a dropped item.
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Classification {
    pub path: String,
    pub name: String,
    pub signature: Signature,
    /// Best first. Empty when nothing fits: the item is then tried as an
    /// app on the tab it was dropped on, for that import's own message.
    pub options: Vec<DropOption>,
    /// Where it goes, when that's settled without asking.
    pub decided: Option<Choice>,
    /// How that was settled: "its contents", "your earlier choice"…
    pub decided_by: Option<String>,
    /// What "do the same next time" covers: ".wp5 files like this".
    pub same_for: String,
}

fn ext_of(name: &str) -> String {
    match name.rsplit_once('.') {
        Some((stem, e)) if !stem.is_empty() => e.to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// A four-letter code as text, other characters as `_`.
fn code(c: &[u8]) -> String {
    c.iter().map(|&b| if b.is_ascii_alphanumeric() || b == b' ' || b == b'#' { b as char } else { '_' }).collect::<String>().trim_end().to_string()
}

const ZIP_DOS: &[&str] = &["exe", "com", "bat"];
const ZIP_MAC: &[&str] = &["sit", "sitx", "sea", "hqx", "cpt", "bin", "dsk", "image", "dc42", "hfv", "hda", "toast"];
const ARCHIVES: &[&str] = &["sit", "sitx", "sea", "hqx", "cpt"];

/// What a dropped item looks like (see [`Signature::content`]).
pub fn signature(path: &Path) -> Signature {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if path.is_dir() {
        let mut guests = Vec::new();
        if !dos::find_programs(path, &name).is_empty() {
            guests.push("dos");
        }
        if !mac::find_programs(path, &name).is_empty() {
            guests.push("mac");
        }
        if !amiga::find_programs(path, &name).is_empty() {
            guests.push("amiga");
        }
        let setup = !cd::setup_slots_in(path, 200).is_empty();
        return Signature { kind: "folder".into(), ext: String::new(), content: programs_label(&guests, setup) };
    }
    let ext = ext_of(&name);
    if ext == "zip" {
        let names = zip_names(path);
        let has = |exts: &[&str]| names.iter().any(|n| exts.contains(&ext_of(n.rsplit('/').next().unwrap_or(n)).as_str()));
        let mut guests = Vec::new();
        if has(ZIP_DOS) {
            guests.push("dos");
        }
        if has(ZIP_MAC) || names.iter().any(|n| n.starts_with("__MACOSX/") || n.rsplit('/').next().is_some_and(|b| b.starts_with("._"))) {
            guests.push("mac");
        }
        if has(&["adf", "adz", "dms", "hdf"]) {
            guests.push("amiga");
        }
        let setup = !cd::setup_slots_named(names.iter().map(String::as_str)).is_empty();
        return Signature { kind: "zip".into(), ext, content: programs_label(&guests, setup) };
    }
    Signature { kind: "file".into(), ext, content: file_content(path) }
}

fn programs_label(guests: &[&str], setup: bool) -> String {
    let programs = if guests.is_empty() { "none".to_string() } else { guests.join("+") };
    format!("programs:{programs}{}", if setup { "+setup" } else { "" })
}

/// A zip's entry names, or none if it isn't one Floppy can read.
fn zip_names(path: &Path) -> Vec<String> {
    let Ok(f) = std::fs::File::open(path) else { return Vec::new() };
    let Ok(z) = zip::ZipArchive::new(f) else { return Vec::new() };
    z.file_names().take(10_000).map(String::from).collect()
}

fn file_content(path: &Path) -> String {
    if let Some(slot) = cd::setup_slot(path) {
        return format!("setup:{}", slot_key(slot));
    }
    let head = library::read_head(path, 512).unwrap_or_default();
    if head.is_empty() {
        return "empty".into();
    }
    if head.starts_with(b"MZ") {
        return "mz".into();
    }
    if head.starts_with(&[0x00, 0x00, 0x03, 0xF3]) {
        return "hunk".into();
    }
    if head.len() >= 12 && head.starts_with(b"FORM") {
        return format!("iff:{}", code(&head[8..12]));
    }
    if head.len() >= 128 {
        if let Some(mb) = mac::read_macbinary(path) {
            return format!("macbinary:{}", code(&mb.finder_info[..4]));
        }
    }
    if let Some(t) = mac::file_type(path) {
        return format!("mac-type:{}", code(&t));
    }
    if amiga::adf_volume(path).is_some() {
        return "adf".into();
    }
    if mac::volume_name(path).is_some() {
        return "hfs".into();
    }
    let printable = head.iter().filter(|&&b| b == b'\n' || b == b'\r' || b == b'\t' || (0x20..0x7F).contains(&b) || b >= 0x80).count();
    if printable * 100 >= head.len() * 95 && !head.contains(&0) {
        "text".into()
    } else {
        "binary".into()
    }
}

fn slot_key(slot: Slot) -> &'static str {
    match slot {
        Slot::MacRom => "mac-rom",
        Slot::MacBoot => "mac-startup-disk",
        Slot::Kickstart => "kickstart",
        Slot::Workbench => "workbench",
    }
}

fn slot_of_key(key: &str) -> Option<Slot> {
    cd::SLOTS.into_iter().find(|s| slot_key(*s) == key)
}

const GUESTS: [GuestOs; 3] = [GuestOs::Dos, GuestOs::MacClassic, GuestOs::Amiga];

/// Every place an item with this signature could go, best first.
/// `missing` is the setup slots still empty; `apps` the library's apps
/// (for the extensions users said they open); `current` the tab it was
/// dropped on.
pub fn options(sig: &Signature, current: GuestOs, missing: &[Slot], apps: &[library::LibraryApp]) -> Vec<DropOption> {
    let mut out: Vec<DropOption> = Vec::new();
    let content = sig.content.as_str();
    match sig.kind.as_str() {
        "folder" | "zip" => {
            let what = if sig.kind == "zip" { "The zip" } else { "The folder" };
            let (programs, setup) = content.strip_prefix("programs:").map(|p| (p.trim_end_matches("+setup"), p.ends_with("+setup"))).unwrap_or(("none", false));
            for part in programs.split('+') {
                match part {
                    "dos" => add(&mut out, Dest::App, GuestOs::Dos, 3, format!("{what} holds DOS programs (.exe, .com or .bat).")),
                    "mac" => add(&mut out, Dest::App, GuestOs::MacClassic, 3, format!("{what} holds a Mac app, disk image or archive.")),
                    "amiga" => add(&mut out, Dest::App, GuestOs::Amiga, 3, format!("{what} holds Amiga disk images or programs.")),
                    _ => {}
                }
            }
            if setup {
                for slot in missing {
                    add(&mut out, Dest::Setup, slot.os(), 2, format!("{what} looks like it holds a {}, which setup still needs.", slot.label()));
                }
            }
            if sig.kind == "zip" && programs == "none" {
                for os in GUESTS {
                    add(&mut out, Dest::Document, os, 1, format!("Keep the zip as it is, as a {} document.", guest_label(os)));
                }
            }
        }
        _ => {
            let ext = sig.ext.as_str();
            if let Some(slot) = content.strip_prefix("setup:").and_then(slot_of_key) {
                if missing.contains(&slot) {
                    add(&mut out, Dest::Setup, slot.os(), 3, format!("It's a {}, which setup still needs.", slot.label()));
                } else {
                    // A ROM is nothing else, so it still goes to setup, which
                    // keeps the one already there. A disk can be an app's.
                    let rom = slot.kind() == library::SystemFile::Rom;
                    add(&mut out, Dest::Setup, slot.os(), if rom { 3 } else { 1 }, format!("It's a {}. You have one already, and Floppy keeps it.", slot.label()));
                }
                match slot {
                    Slot::MacBoot => add(&mut out, Dest::App, GuestOs::MacClassic, 2, "It's a Mac disk: as an app, it's mounted beside the startup disk.".into()),
                    Slot::Workbench => add(&mut out, Dest::App, GuestOs::Amiga, 2, "It's an Amiga floppy: as an app, it boots on its own.".into()),
                    _ => {}
                }
            }
            match ext {
                "exe" if content == "mz" => add(&mut out, Dest::App, GuestOs::Dos, 3, "It's a DOS program (its header says so).".into()),
                "exe" | "com" | "bat" => add(&mut out, Dest::App, GuestOs::Dos, 2, format!("It's named like a DOS program (.{ext}).")),
                _ => {}
            }
            if content == "hunk" {
                add(&mut out, Dest::App, GuestOs::Amiga, 3, "It's an Amiga program (its header says so).".into());
            }
            if content == "adf" {
                add(&mut out, Dest::App, GuestOs::Amiga, 3, "It's an AmigaDOS floppy image.".into());
            } else if amiga::is_disk_image(&format!("x.{ext}")) {
                add(&mut out, Dest::App, GuestOs::Amiga, 2, format!("It's named like an Amiga disk image (.{ext})."));
            }
            if content == "hfs" {
                add(&mut out, Dest::App, GuestOs::MacClassic, 3, "It's a Mac disk image (an HFS volume).".into());
            } else if mac::is_disk_image(&format!("x.{ext}")) {
                add(&mut out, Dest::App, GuestOs::MacClassic, 2, format!("It's named like a Mac disk image (.{ext})."));
            }
            if ARCHIVES.contains(&ext) {
                add(&mut out, Dest::App, GuestOs::MacClassic, 3, format!("It's a Mac archive (.{ext}), expanded inside the Mac."));
            }
            if let Some(t) = content.strip_prefix("mac-type:").or_else(|| content.strip_prefix("macbinary:")) {
                if t == "APPL" {
                    add(&mut out, Dest::App, GuestOs::MacClassic, 3, "It's a Mac app (its Finder type is APPL).".into());
                } else {
                    let handler = handlers::HANDLERS.iter().find(|h| h.os == GuestOs::MacClassic && h.types.contains(&t));
                    match handler {
                        Some(h) => add(&mut out, Dest::Document, GuestOs::MacClassic, 3, format!("It's a Mac document of type {t}, which {} opens.", h.name)),
                        None => add(&mut out, Dest::Document, GuestOs::MacClassic, 2, format!("It's a Mac document (type {t}).")),
                    }
                }
            }
            if let Some(t) = content.strip_prefix("iff:") {
                match handlers::HANDLERS.iter().find(|h| h.os == GuestOs::Amiga && h.types.contains(&t)) {
                    Some(h) => add(&mut out, Dest::Document, GuestOs::Amiga, 3, format!("It's an Amiga IFF {t} file, which {} opens.", h.name)),
                    None => add(&mut out, Dest::Document, GuestOs::Amiga, 2, format!("It's an Amiga IFF file ({t}).")),
                }
            }
            if !ext.is_empty() && !matches!(ext, "exe" | "com" | "bat") {
                let up = ext.to_ascii_uppercase();
                if let Some(h) = handlers::HANDLERS.iter().find(|h| h.os == GuestOs::Dos && handlers::opens_ext(h, &up)) {
                    add(&mut out, Dest::Document, GuestOs::Dos, 2, format!("{} opens .{up} files.", h.name));
                } else if let Some(a) = apps.iter().find(|a| a.os == GuestOs::Dos && a.opens.iter().any(|e| e.eq_ignore_ascii_case(&up))) {
                    add(&mut out, Dest::Document, GuestOs::Dos, 2, format!("You said {} opens .{up} files.", a.name));
                }
            }
            // Nothing in it says where: it could be any guest's document,
            // and the tab it was dropped on says which, unless its name
            // points at another guest.
            if !out.iter().any(|o| o.score == 3) {
                for os in GUESTS {
                    add(&mut out, Dest::Document, os, 1, format!("An old file to keep as a {} document.", guest_label(os)));
                }
                if out.iter().filter(|o| o.score >= 2).all(|o| o.choice.to == Dest::Document) {
                    add(&mut out, Dest::Document, current, 2, format!("You dropped it on the {} tab.", guest_label(current)));
                }
            }
        }
    }
    // Best first; among equals, the tab it was dropped on, then the order
    // of the tabs.
    let rank = |os: GuestOs| (os != current, GUESTS.iter().position(|g| *g == os));
    out.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| rank(a.choice.os).cmp(&rank(b.choice.os))));
    out
}

/// Adds an option, or raises one already there to `score`.
fn add(out: &mut Vec<DropOption>, to: Dest, os: GuestOs, score: u8, why: String) {
    let choice = Choice { to, os };
    match out.iter_mut().find(|o| o.choice == choice) {
        Some(o) if o.score >= score => {}
        Some(o) => *o = DropOption { choice, label: choice.label(), score, why },
        None => out.push(DropOption { choice, label: choice.label(), score, why }),
    }
}

/// The option that wins on its own: the only one with the top score, at
/// 2 or more.
pub fn clear_winner(options: &[DropOption]) -> Option<Choice> {
    let top = options.first()?;
    (top.score >= 2 && options.get(1).is_none_or(|o| o.score < top.score)).then_some(top.choice)
}

/// An answer the user gave in the dialog.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DropAnswer {
    pub signature: Signature,
    pub choice: Choice,
    /// What the dialog offered.
    pub offered: Vec<Choice>,
    /// Unix seconds.
    pub answered: u64,
    /// Whether to go the same way next time without asking.
    #[serde(default)]
    pub remember: bool,
}

pub fn load_answers(library: &Library) -> Vec<DropAnswer> {
    std::fs::read(library.drop_answers_path()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
}

/// Keeps an answer, for next time and for the next Export Findings.
pub fn record(library: &Library, signature: Signature, choice: Choice, offered: Vec<Choice>, remember: bool) -> Result<(), String> {
    if !signature.is_valid() {
        return Err("That isn't a drop Floppy judged.".into());
    }
    let mut all = load_answers(library);
    let answered = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
    // A newer answer for the same kind of item replaces the remembered one.
    if remember {
        for a in all.iter_mut().filter(|a| a.signature == signature) {
            a.remember = false;
        }
    }
    all.push(DropAnswer { signature, choice, offered, answered, remember });
    let path = library.drop_answers_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec_pretty(&all).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("Couldn't keep your choice: {e}"))
}

/// A choice for a kind of item, with how many people made it: a row of
/// Floppy's rules, or learned from findings.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DropRule {
    pub signature: Signature,
    pub choice: Choice,
    pub answers: u32,
}

/// Floppy's rules: the "Drop choices" table in `docs/file-handling.md`.
pub fn rules() -> &'static [DropRule] {
    static RULES: OnceLock<Vec<DropRule>> = OnceLock::new();
    RULES.get_or_init(|| parse_rules(include_str!("../../docs/file-handling.md")).unwrap_or_default())
}

/// `| Kind | Extension | Contents | Goes to | Answers | Last answered |`.
pub(crate) fn parse_rules(doc: &str) -> Result<Vec<DropRule>, String> {
    handlers::table_rows(doc, "drops")?
        .into_iter()
        .map(|cells| {
            let [kind, ext, content, to, answers, _last] = &cells[..] else {
                return Err(format!("a drop choice needs 6 cells: {cells:?}"));
            };
            let signature = Signature { kind: kind.clone(), ext: ext.trim_start_matches('.').to_ascii_lowercase(), content: content.clone() };
            if !signature.is_valid() {
                return Err(format!("not a valid drop signature: {cells:?}"));
            }
            Ok(DropRule {
                signature,
                choice: Choice::parse(to).ok_or(format!("unknown destination {to:?}"))?,
                answers: answers.parse().map_err(|_| format!("not a number {answers:?}"))?,
            })
        })
        .collect()
}

/// The choice most answers made for `sig` among `offered`, if one leads.
fn consensus(rules: &[DropRule], sig: &Signature, offered: &[Choice]) -> Option<Choice> {
    let mut counts: Vec<(Choice, u32)> = Vec::new();
    for r in rules.iter().filter(|r| r.signature == *sig && offered.contains(&r.choice)) {
        match counts.iter_mut().find(|(c, _)| *c == r.choice) {
            Some((_, n)) => *n = n.saturating_add(r.answers),
            None => counts.push((r.choice, r.answers)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    match counts.as_slice() {
        [(c, n), rest @ ..] if *n > 0 && rest.first().is_none_or(|(_, m)| m < n) => Some(*c),
        _ => None,
    }
}

/// What Floppy makes of a dropped item on the `current` tab.
pub fn classify(library: &Library, path: &Path, current: GuestOs) -> Result<Classification, String> {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).ok_or("That path has no file name.")?;
    if !path.exists() {
        return Err(format!("{name} is gone."));
    }
    let signature = signature(path);
    let missing = cd::missing_slots(library)?;
    let options = options(&signature, current, &missing, &library.list()?);
    let offered = offered(&options);
    let (decided, decided_by) = if options.is_empty() {
        (Some(Choice { to: Dest::App, os: current }), Some("nothing else fits".to_string()))
    } else if let Some(c) = clear_winner(&options) {
        let by = if options[0].score == 3 { "its contents" } else { "its name and where you dropped it" };
        (Some(c), Some(by.to_string()))
    } else if let Some(a) = load_answers(library).into_iter().rev().find(|a| a.remember && a.signature == signature && offered.contains(&a.choice)) {
        (Some(a.choice), Some("your earlier choice".to_string()))
    } else if let Some(c) = consensus(rules(), &signature, &offered) {
        (Some(c), Some("Floppy's rules".to_string()))
    } else if let Some(c) = consensus(&learned::current().drop_rules, &signature, &offered) {
        (Some(c), Some("what other Floppy users chose".to_string()))
    } else {
        (None, None)
    };
    let same_for = signature.describe();
    Ok(Classification { path: path.to_string_lossy().into_owned(), name, signature, options, decided, decided_by, same_for })
}

/// Every choice a set of options holds (for the findings' `offered`).
pub fn offered(options: &[DropOption]) -> Vec<Choice> {
    let set: BTreeSet<Choice> = options.iter().map(|o| o.choice).collect();
    set.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::TempDir;
    use std::fs;

    fn file(t: &TempDir, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let p = t.path().join(name);
        fs::write(&p, bytes).unwrap();
        p
    }

    fn winner(t: &TempDir, name: &str, bytes: &[u8], current: GuestOs) -> Option<Choice> {
        let sig = signature(&file(t, name, bytes));
        clear_winner(&options(&sig, current, &cd::SLOTS, &[]))
    }

    const DOS_APP: Choice = Choice { to: Dest::App, os: GuestOs::Dos };
    const DOS_DOC: Choice = Choice { to: Dest::Document, os: GuestOs::Dos };
    const AMIGA_APP: Choice = Choice { to: Dest::App, os: GuestOs::Amiga };
    const AMIGA_DOC: Choice = Choice { to: Dest::Document, os: GuestOs::Amiga };

    #[test]
    fn contents_win_wherever_it_was_dropped() {
        let t = TempDir::new();
        assert_eq!(winner(&t, "GAME.EXE", b"MZ\x90\x00", GuestOs::Amiga), Some(DOS_APP));
        assert_eq!(winner(&t, "Deluxe", &[0, 0, 3, 0xF3, 0, 0], GuestOs::Dos), Some(AMIGA_APP));
        assert_eq!(winner(&t, "Sunset.iff", b"FORM\0\0\0\x04ILBM", GuestOs::Dos), Some(AMIGA_DOC));
        // A known DOS document type on the DOS tab.
        assert_eq!(winner(&t, "LETTER.WP5", b"\xffWPC", GuestOs::Dos), Some(DOS_DOC));
        // An unknown file goes to the tab it was dropped on.
        assert_eq!(winner(&t, "README", b"Read me first.\n", GuestOs::Amiga), Some(AMIGA_DOC));
    }

    #[test]
    fn no_clear_winner_asks() {
        let t = TempDir::new();
        // A WordPerfect document dropped on the Amiga tab: DOS by its
        // name, the Amiga by where it was dropped.
        let sig = signature(&file(&t, "LETTER.WP5", b"\xffWPC"));
        let opts = options(&sig, GuestOs::Amiga, &cd::SLOTS, &[]);
        assert_eq!(clear_winner(&opts), None);
        assert_eq!(opts.iter().filter(|o| o.score == 2).map(|o| o.choice).collect::<Vec<_>>(), [AMIGA_DOC, DOS_DOC]);
        // A zip with nothing recognizable in it.
        let zip = t.path().join("stuff.zip");
        let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        w.start_file("notes.txt", zip::write::SimpleFileOptions::default()).unwrap();
        std::io::Write::write_all(&mut w, b"hello").unwrap();
        w.finish().unwrap();
        let sig = signature(&zip);
        assert_eq!(sig, Signature { kind: "zip".into(), ext: "zip".into(), content: "programs:none".into() });
        assert_eq!(clear_winner(&options(&sig, GuestOs::Dos, &cd::SLOTS, &[])), None);
    }

    #[test]
    fn a_setup_disk_goes_to_setup_until_its_slot_is_filled() {
        // A Workbench floppy while Workbench is missing: setup, or an app
        // disk that boots on its own.
        let sig = Signature { kind: "file".into(), ext: "adf".into(), content: "setup:workbench".into() };
        let opts = options(&sig, GuestOs::Amiga, &[Slot::Workbench], &[]);
        assert_eq!(clear_winner(&opts), Some(Choice { to: Dest::Setup, os: GuestOs::Amiga }));
        // Once Workbench is set up, it's an app disk.
        let opts = options(&sig, GuestOs::Amiga, &[], &[]);
        assert_eq!(clear_winner(&opts), Some(AMIGA_APP));
    }

    #[test]
    fn answers_are_remembered_and_rules_parse() {
        let t = TempDir::new();
        let lib = Library::new(t.path().join("lib"));
        let path = file(&t, "LETTER.WP5", b"\xffWPC");
        let c = classify(&lib, &path, GuestOs::Amiga).unwrap();
        assert_eq!(c.decided, None);
        record(&lib, c.signature.clone(), DOS_DOC, offered(&c.options), true).unwrap();
        let again = classify(&lib, &path, GuestOs::Amiga).unwrap();
        assert_eq!((again.decided, again.decided_by.as_deref()), (Some(DOS_DOC), Some("your earlier choice")));
        // An answer not to remember is kept for the findings only.
        let other = file(&t, "a.q7x", b"\x01\x02binary\x00");
        let c = classify(&lib, &other, GuestOs::Dos).unwrap();
        assert!(c.decided.is_some(), "an unknown file on the DOS tab stays on DOS");
        assert_eq!(load_answers(&lib).len(), 1);

        let doc = "<!-- drops:start -->\n| Kind | Extension | Contents | Goes to | Answers | Last answered |\n|---|---|---|---|---|---|\n\
                   | file | .wp5 | binary | Document, DOS | 3 | 2026-09-27 |\n| file | .wp5 | binary | Document, Amiga | 1 | 2026-09-27 |\n<!-- drops:end -->";
        let rules = parse_rules(doc).unwrap();
        let sig = Signature { kind: "file".into(), ext: "wp5".into(), content: "binary".into() };
        assert_eq!(consensus(&rules, &sig, &[AMIGA_DOC, DOS_DOC]), Some(DOS_DOC));
        assert_eq!(consensus(&rules, &sig, &[AMIGA_DOC]), Some(AMIGA_DOC));
        assert!(parse_rules(&doc.replace("Document, DOS", "Somewhere")).is_err());
        // The shipped table parses.
        parse_rules(include_str!("../../docs/file-handling.md")).unwrap();
    }

    #[test]
    fn signatures_hold_no_names() {
        let t = TempDir::new();
        let dir = t.path().join("Secret Project");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SECRET.EXE"), b"MZ").unwrap();
        let sig = signature(&dir);
        assert_eq!(sig, Signature { kind: "folder".into(), ext: String::new(), content: "programs:dos".into() });
        assert!(sig.is_valid());
        assert!(!Signature { kind: "file".into(), ext: "a|b".into(), content: String::new() }.is_valid());
    }
}
