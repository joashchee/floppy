//! The maintainers' signing key for Floppy AI knowledge packs
//! (`docs/floppy-ai.md`, "Pack keys"). A build tool, never part of the
//! app: Floppy itself only verifies (ai.rs).
//!
//!     cargo run --example ai-pack-key -- new <keyfile>
//!         Makes a key: the secret goes in <keyfile> (readable by you
//!         only), and the public key is printed, to add to "Pack keys".
//!         Keep <keyfile> out of every repo.
//!     cargo run --example ai-pack-key -- public <keyfile>
//!         Prints the public key again.
//!     cargo run --example ai-pack-key -- sign <keyfile> <file>
//!         Prints the signature of <file> (floppy-findings.json), in hex.
//!         `scripts/make-ai-pack.py --sign <keyfile>` runs this.

use std::io::Read;

use ed25519_dalek::{Signer, SigningKey};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_key(path: &str) -> SigningKey {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| die(&format!("Couldn't read {path}: {e}")));
    let text = text.trim();
    if text.len() != 64 {
        die(&format!("{path} isn't a Floppy AI pack key."));
    }
    let mut seed = [0u8; 32];
    for (i, b) in seed.iter_mut().enumerate() {
        *b = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap_or_else(|_| die(&format!("{path} isn't a Floppy AI pack key.")));
    }
    SigningKey::from_bytes(&seed)
}

fn die(msg: &str) -> ! {
    eprintln!("ai-pack-key: {msg}");
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["new", path] => {
            if std::path::Path::new(path).exists() {
                die(&format!("{path} exists already: not replacing a key."));
            }
            let mut seed = [0u8; 32];
            std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut seed)).unwrap_or_else(|e| die(&format!("No randomness: {e}")));
            let key = SigningKey::from_bytes(&seed);
            let mut opts = std::fs::OpenOptions::new();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut opts, 0o600);
            let mut f = opts.open(path).unwrap_or_else(|e| die(&format!("Couldn't create {path}: {e}")));
            std::io::Write::write_all(&mut f, format!("{}\n", hex(&seed)).as_bytes()).unwrap_or_else(|e| die(&e.to_string()));
            println!("{}", hex(key.verifying_key().as_bytes()));
            eprintln!("Secret key in {path}. Add the public key above to docs/floppy-ai.md's Pack keys, and keep {path} out of every repo.");
        }
        ["public", path] => println!("{}", hex(read_key(path).verifying_key().as_bytes())),
        ["sign", path, file] => {
            let data = std::fs::read(file).unwrap_or_else(|e| die(&format!("Couldn't read {file}: {e}")));
            println!("{}", hex(&read_key(path).sign(&data).to_bytes()));
        }
        _ => die("usage: ai-pack-key new <keyfile> | public <keyfile> | sign <keyfile> <file>"),
    }
}
