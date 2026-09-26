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
    let (bin, _) = Emulator::DosboxStaging.locate(None, None).expect("run scripts/fetch-dosbox.sh first");
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
        args: None,
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

/// A 36-byte .COM program that recreates, empty, the file named on its
/// command line (proof the document argument arrived):
/// `mov si,81h / skip: lodsb / cmp al,' ' / je skip / dec si / mov dx,si /
/// end: lodsb / cmp al,0Dh / jne end / mov byte [si-1],0 / mov ah,3Ch /
/// xor cx,cx / int 21h / jc done / mov bx,ax / mov ah,3Eh / int 21h /
/// done: int 20h`.
const EMPTY_THE_ARG: &[u8] = &[
    0xBE, 0x81, 0x00, 0xAC, 0x3C, 0x20, 0x74, 0xFB, 0x4E, 0x89, 0xF2, 0xAC, 0x3C, 0x0D, 0x75, 0xFB, 0xC6, 0x44, 0xFF, 0x00,
    0xB4, 0x3C, 0x31, 0xC9, 0xCD, 0x21, 0x72, 0x06, 0x89, 0xC3, 0xB4, 0x3E, 0xCD, 0x21, 0xCD, 0x20,
];

#[test]
#[ignore]
fn document_opens_in_its_app_and_the_change_is_listed() {
    use crate::documents::{changes, dos_openers, dos_path, snapshot};

    let (bin, _) = Emulator::DosboxStaging.locate(None, None).expect("run scripts/fetch-dosbox.sh first");
    std::env::set_var("SDL_VIDEODRIVER", "dummy");
    std::env::set_var("SDL_AUDIODRIVER", "dummy");
    let t = TempDir::new();
    let src = t.path().join("Emptier");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("EMPTY.COM"), EMPTY_THE_ARG).unwrap();
    let lib = Library::new(t.path().join("library"));
    let app = lib.import(GuestOs::Dos, &src).unwrap();
    lib.set_opens(&app.id, &["TXT".to_string()]).unwrap();
    let doc_src = t.path().join("Meeting Notes.txt");
    std::fs::write(&doc_src, b"hello").unwrap();
    let doc = lib.import_document(GuestOs::Dos, &doc_src).unwrap();

    // The app is offered for the document, and runs with its DOS path.
    let opener = dos_openers(&doc.file, None, &lib.list().unwrap(), &[]).into_iter().next().expect("no opener");
    assert_eq!(opener.app_id, app.id);
    let mount_root = lib.os_root(GuestOs::Dos);
    let args = dos_path(&doc);
    let conf = dosbox_conf(&Launch {
        mount_root: &mount_root,
        app_dir: &app.dir,
        program: Some(&opener.program),
        exit_after: true,
        args: Some(&args),
    })
    .unwrap();
    let conf_path = t.path().join("run.conf");
    std::fs::write(&conf_path, conf).unwrap();

    let before = snapshot(&mount_root);
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
    assert_eq!(std::fs::read(lib.document_path(&doc).unwrap()).unwrap(), b"", "the program didn't get the document");
    let changed = changes(&before, &snapshot(&mount_root));
    assert_eq!(changed.iter().map(|c| (c.path.as_str(), c.new)).collect::<Vec<_>>(), [(doc.file.as_str(), false)]);
}
