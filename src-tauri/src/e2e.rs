//! End-to-end: import a real DOS program and run it in DOSBox Staging.
//! Ignored by default because it needs the fetched DOSBox
//! (`scripts/fetch-dosbox.sh`). Run with
//! `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored`.
//! It runs headless (SDL's dummy drivers), so it works without a display,
//! as in CI or a sandboxed shell.

use std::time::{Duration, Instant};

use crate::dos::{dosbox_conf, Launch};
use crate::emulator::Emulator;
use crate::library::{GuestOs, Library};
use crate::testutil::TempDir;

/// A 25-byte .COM program that creates an empty RAN.TXT and exits:
/// `mov ah,3Ch / xor cx,cx / mov dx,name / int 21h / mov bx,ax /
/// mov ah,3Eh / int 21h / int 20h / name db "RAN.TXT",0`.
const MAKE_RAN_TXT: &[u8] = &[
    0xB4, 0x3C, 0x31, 0xC9, 0xBA, 0x11, 0x01, 0xCD, 0x21, 0x89, 0xC3, 0xB4, 0x3E, 0xCD, 0x21, 0xCD, 0x20, b'R', b'A',
    b'N', b'.', b'T', b'X', b'T', 0,
];

#[test]
#[ignore]
fn program_runs_in_dosbox_and_dosbox_exits() {
    let (bin, _) = Emulator::DosboxStaging.locate(None).expect("run scripts/fetch-dosbox.sh first");
    // Inherited by DOSBox. Only this (ignored, run-alone) test sets them.
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    let t = TempDir::new();
    let src = t.path().join("Test App");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("MAKERAN.COM"), MAKE_RAN_TXT).unwrap();

    let lib = Library::new(t.path().join("library"));
    let app = lib.import(GuestOs::Dos, &src).unwrap();
    let mount_root = lib.os_root(GuestOs::Dos);
    let conf = dosbox_conf(&Launch {
        mount_root: &mount_root,
        app_dir: &app.dir,
        program: app.program.as_deref(),
        exit_after: true,
    })
    .unwrap();
    let conf_path = t.path().join("run.conf");
    std::fs::write(&conf_path, conf).unwrap();

    let mut child = Emulator::DosboxStaging.spawn(&bin, &conf_path).unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            panic!("DOSBox didn't exit within 30 s");
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    assert!(status.success(), "DOSBox exited with {status}");
    assert!(mount_root.join(&app.dir).join("RAN.TXT").exists(), "the program didn't run");
}
