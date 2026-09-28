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

/// The public keys that sign official knowledge packs: "Pack keys" in
/// `docs/floppy-ai.md` (`| Key | Added | Note |`, 64 hex digits each).
/// Keys are added there, never retired silently: a key that's out of use
/// is deleted in a release, and packs it signed then count as unsigned.
pub fn parse_pack_keys(doc: &str) -> Result<Vec<[u8; 32]>, String> {
    handlers::table_rows(doc, "pack-keys")?
        .into_iter()
        .map(|cells| {
            let key = cells.first().ok_or("an empty pack key row")?;
            let bytes = unhex(key).filter(|b| b.len() == 32).ok_or(format!("not a 32-byte key in hex: {key:?}"))?;
            let key: [u8; 32] = bytes.try_into().expect("32 bytes");
            ed25519_dalek::VerifyingKey::from_bytes(&key).map_err(|_| "not an Ed25519 public key".to_string())?;
            Ok(key)
        })
        .collect()
}

pub fn pack_keys() -> &'static [[u8; 32]] {
    static KEYS: OnceLock<Vec<[u8; 32]>> = OnceLock::new();
    KEYS.get_or_init(|| parse_pack_keys(include_str!("../../docs/floppy-ai.md")).unwrap_or_default())
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// The keys a pack may be signed with: "Pack keys", plus in tests only a
/// fixed test key, which no release build knows.
pub fn trusted_keys() -> Vec<[u8; 32]> {
    #[allow(unused_mut)]
    let mut keys = pack_keys().to_vec();
    #[cfg(test)]
    keys.push(*test_key().verifying_key().as_bytes());
    keys
}

/// A signing key for tests: its secret is in this file, so it must never
/// be listed in "Pack keys".
#[cfg(test)]
pub fn test_key() -> ed25519_dalek::SigningKey {
    ed25519_dalek::SigningKey::from_bytes(&[42u8; 32])
}

/// Whether `signature` (hex) signs `data` with one of `keys`. Strict
/// verification: no malleable signatures, no weak keys.
pub fn verify(keys: &[[u8; 32]], data: &[u8], signature: &str) -> bool {
    let Some(sig) = unhex(signature).and_then(|b| <[u8; 64]>::try_from(b).ok()) else { return false };
    let sig = ed25519_dalek::Signature::from_bytes(&sig);
    keys.iter().filter_map(|k| ed25519_dalek::VerifyingKey::from_bytes(k).ok()).any(|k| k.verify_strict(data, &sig).is_ok())
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
    fn packs_verify_only_with_a_listed_key() {
        use ed25519_dalek::{Signer, SigningKey};
        let key = SigningKey::from_bytes(&[7u8; 32]);
        let other = SigningKey::from_bytes(&[8u8; 32]);
        let keys = [*key.verifying_key().as_bytes()];
        let data = br#"{"format":"floppy-findings"}"#;
        let hex = |b: &[u8]| b.iter().map(|x| format!("{x:02x}")).collect::<String>();
        let sig = hex(&key.sign(data).to_bytes());
        assert!(verify(&keys, data, &sig));
        assert!(!verify(&keys, b"{\"format\":\"floppy-findings \"}", &sig), "a changed file");
        assert!(!verify(&keys, data, &hex(&other.sign(data).to_bytes())), "another key");
        assert!(!verify(&[], data, &sig), "no keys, nothing is official");
        assert!(!verify(&keys, data, "zz"));
        // The table parses, and refuses what isn't a key.
        let doc = |k: &str| format!("<!-- pack-keys:start -->\n| Key | Added | Note |\n|---|---|---|\n| {k} | 2026-09-28 | test |\n<!-- pack-keys:end -->");
        assert_eq!(parse_pack_keys(&doc(&hex(&keys[0]))).unwrap(), keys);
        assert!(parse_pack_keys(&doc("abc")).is_err());
        let shipped = parse_pack_keys(include_str!("../../docs/floppy-ai.md")).unwrap();
        assert!(!shipped.contains(test_key().verifying_key().as_bytes()), "the test key's secret is public: never list it");
    }

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
