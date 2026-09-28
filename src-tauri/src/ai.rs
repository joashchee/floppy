//! Floppy AI's version (`docs/floppy-ai.md`): the version of what Floppy
//! knows about old files, apps and setup, separate from the app's own
//! version. The build's is the top row of that document's table, read
//! when Floppy is built. A knowledge pack learned from (learned.rs) can
//! make this Floppy's newer, without a new release.

use std::sync::OnceLock;

use serde::Serialize;

use crate::handlers;
use crate::learned;

/// A row of the versions table.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiVersion {
    pub version: u32,
    /// `YYYY-MM-DD`.
    pub date: String,
    pub what: String,
}

/// `| AI version | Date | What changed |`, newest first. Versions must go
/// down row by row, so a bad merge fails the tests.
pub fn parse_versions(doc: &str) -> Result<Vec<AiVersion>, String> {
    let rows: Vec<AiVersion> = handlers::table_rows(doc, "ai-versions")?
        .into_iter()
        .map(|cells| {
            let [version, date, what] = &cells[..] else {
                return Err(format!("an AI version needs 3 cells: {cells:?}"));
            };
            let d = date.as_bytes();
            if d.len() != 10 || d[4] != b'-' || d[7] != b'-' {
                return Err(format!("not a date: {date:?}"));
            }
            Ok(AiVersion { version: version.parse().map_err(|_| format!("not a version number: {version:?}"))?, date: date.clone(), what: what.clone() })
        })
        .collect::<Result<_, _>>()?;
    if rows.is_empty() {
        return Err("no AI version".into());
    }
    if rows.windows(2).any(|w| w[0].version <= w[1].version) {
        return Err("AI versions must be newest first, each higher than the one below".into());
    }
    Ok(rows)
}

/// This build's AI version.
pub fn builtin() -> &'static AiVersion {
    static BUILTIN: OnceLock<AiVersion> = OnceLock::new();
    BUILTIN.get_or_init(|| {
        parse_versions(include_str!("../../docs/floppy-ai.md"))
            .ok()
            .and_then(|v| v.into_iter().next())
            .unwrap_or(AiVersion { version: 0, date: String::new(), what: String::new() })
    })
}

/// What the header and About show.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AiInfo {
    /// This Floppy's AI version: the build's, or a newer pack's.
    pub version: u32,
    pub date: String,
    /// The build's.
    pub builtin: u32,
    /// Set when a newer knowledge pack raised it.
    pub from_pack: bool,
    /// How many findings files it learned from.
    pub learned_from: usize,
}

pub fn info() -> AiInfo {
    let b = builtin();
    let l = learned::current();
    match l.ai_version.filter(|v| *v > b.version) {
        Some(v) => AiInfo { version: v, date: l.ai_date.clone(), builtin: b.version, from_pack: true, learned_from: l.sources.len() },
        None => AiInfo { version: b.version, date: b.date.clone(), builtin: b.version, from_pack: false, learned_from: l.sources.len() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_versions_parse_newest_first() {
        let v = parse_versions(include_str!("../../docs/floppy-ai.md")).unwrap();
        assert_eq!(builtin().version, v[0].version);
        assert!(builtin().version >= 1);
        let doc = |rows: &str| format!("<!-- ai-versions:start -->\n| AI version | Date | What changed |\n|---|---|---|\n{rows}<!-- ai-versions:end -->");
        assert!(parse_versions(&doc("| 2 | 2026-10-01 | b |\n| 1 | 2026-09-27 | a |\n")).is_ok());
        assert!(parse_versions(&doc("| 1 | 2026-09-27 | a |\n| 2 | 2026-10-01 | b |\n")).is_err());
        assert!(parse_versions(&doc("| x | 2026-09-27 | a |\n")).is_err());
        assert!(parse_versions(&doc("")).is_err());
    }
}
