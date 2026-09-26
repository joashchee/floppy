//! SHA-1s of known-good system ROM dumps, so the missing-files list can
//! say what Floppy is looking for by content as well as by name. A user's
//! ROM is often stored under a name no list could guess; a tool that has
//! hashed the user's files can still find it by one of these.
//!
//! SHA-1 because that's what ROM databases publish. These are facts about
//! the files, not the files themselves (rule 3). Floppy itself never
//! relies on them: it still recognizes every file by its contents
//! (`mac::is_basilisk_rom`, `amiga::identify_kickstart`), so a dump that
//! isn't listed here still works when it's imported directly.

/// A ROM dump's SHA-1, its size in bytes, and what it's from.
pub struct KnownRom {
    pub sha1: &'static str,
    pub size: u32,
    pub label: &'static str,
}

/// Mac ROMs Basilisk II can use: 32-bit clean, single-file 512 KB or 1 MB
/// dumps. From MAME's Apple drivers (`src/mame/apple/mac*.cpp`), which
/// name each dump by the checksum in its first long word (shown in
/// brackets). The IIci's ROM is left out: MAME only lists it as four chips.
pub const MAC_ROMS: &[KnownRom] = &[
    KnownRom { sha1: "f2a9ce387019bf272c6e3459d961b30f28942ac5", size: 1048576, label: "Quadra 610/650/800, Centris 610/650 (F1ACAD13)" },
    KnownRom { sha1: "031f13bfd726a70cfe4c73c5967861bc77297a79", size: 1048576, label: "Quadra 800, Centris 610/650, earlier (F1A6F343)" },
    KnownRom { sha1: "7a8ee468d16e64f2ad10cb8d1a45e6f07cc9e212", size: 1048576, label: "Quadra 700/900, PowerBook 140/170 (420DBFF3)" },
    KnownRom { sha1: "d61dba4a2d2cf9048244b713eaa294100063658d", size: 1048576, label: "Quadra 950 (3DC27823)" },
    KnownRom { sha1: "1d833125adf553a50f5994746c2c01aa5a1dbbf2", size: 1048576, label: "Quadra 605, LC 475 (FF7439EE)" },
    KnownRom { sha1: "47cd505b6a7c46e5c0ffa29f0d5037c83e94a02f", size: 1048576, label: "Quadra 630, LC 580 (06684214)" },
    KnownRom { sha1: "f48a8adf06bce50beee033d0d814da0e5e916d08", size: 1048576, label: "LC 580, later (064DC91D)" },
    KnownRom { sha1: "c77df3220c861f37a2c553b6ee9241b202dfdffc", size: 1048576, label: "LC III (ECBBC41C)" },
    KnownRom { sha1: "c54ee2f45020a4adeb7451adce04cd6e5fb69790", size: 1048576, label: "LC 520/550 (EDE66CBD)" },
    KnownRom { sha1: "74975c60d3a560fac9ad63125bb65a750fceaede", size: 1048576, label: "Macintosh TV (EAF1678D)" },
    KnownRom { sha1: "fd9e852e2d77fe17287ba678709b9334d4d74f1e", size: 1048576, label: "Color Classic (ECD99DC0)" },
    KnownRom { sha1: "560ce203d65178657ad09d03f532f86fa512bb40", size: 1048576, label: "IIvi/IIvx (4957EB49)" },
    KnownRom { sha1: "f923de4125aae810796527ff6e25364cf1d54eec", size: 524288, label: "IIsi (36B7FB6C)" },
    KnownRom { sha1: "9fba3d4f672a630745d65788b1d1119afa2c6728", size: 524288, label: "IIfx (4147DD77)" },
    KnownRom { sha1: "6bef5853ae736f3f06c2b4e79772f65910c3b7d4", size: 524288, label: "LC (350EACF0)" },
    KnownRom { sha1: "c0510682ae6d973652d7e17f3c3b27629c47afac", size: 1048576, label: "PowerBook 160/165c/180/180c (E33B2724)" },
    KnownRom { sha1: "ef56fadc8adbe978f9d91fdf1543f66cb8468390", size: 1048576, label: "PowerBook 150 (FBA22562)" },
    KnownRom { sha1: "ed1371c97117a5884da4a6605ecfc5abed48ae5a", size: 1048576, label: "PowerBook Duo 210/230 (ECFA989B)" },
    KnownRom { sha1: "be7bd9637203e4513b896146ddfc85c37817d131", size: 1048576, label: "PowerBook Duo 270c (0024D346)" },
    KnownRom { sha1: "d49dd69cf038784b8849793ad3c0e62c2d11f653", size: 1048576, label: "PowerBook Duo 280/280c (015621D7)" },
];

/// Kickstart ROMs: every unencrypted Kickstart in FS-UAE's ROM table
/// (`rommgr.cpp`). Amiga Forever's encrypted copies hash differently and
/// are found through `rom.key` instead.
pub const KICKSTARTS: &[KnownRom] = &[
    KnownRom { sha1: "00c15406beb4b8ab1a16aa66c05860e1a7c1ad79", size: 262144, label: "Kickstart v1.0 (A1000)(NTSC)" },
    KnownRom { sha1: "4192c505d130f446b2ada6bdc91dae730acafb4c", size: 262144, label: "Kickstart v1.1 (A1000)(NTSC)" },
    KnownRom { sha1: "16df8b5fd524c5a1c7584b2457ac15aff9e3ad6d", size: 262144, label: "Kickstart v1.1 (A1000)(PAL)" },
    KnownRom { sha1: "6a7bfb5dbd6b8f179f03da84d8d9528267b6273b", size: 262144, label: "Kickstart v1.2 (A1000)" },
    KnownRom { sha1: "11f9e62cf299f72184835b7b2a70a16333fc0d88", size: 262144, label: "Kickstart v1.2 (A500,A1000,A2000)" },
    KnownRom { sha1: "891e9a547772fe0c6c19b610baf8bc4ea7fcb785", size: 262144, label: "Kickstart v1.3 (A500,A1000,A2000)" },
    KnownRom { sha1: "c39bd9094d4e5f4e28c1411f3086950406062e87", size: 262144, label: "Kickstart v1.3 (A3000)(SK)" },
    KnownRom { sha1: "f76316bf36dff14b20fa349ed02e4b11dd932b07", size: 524288, label: "Kickstart v1.4 (A3000)" },
    KnownRom { sha1: "c5839f5cb98a7a8947065c3ed2f14f5f42e334a1", size: 524288, label: "Kickstart v2.04 (A500+)" },
    KnownRom { sha1: "87508de834dc7eb47359cede72d2e3c8a2e5d8db", size: 524288, label: "Kickstart v2.05 (A600)" },
    KnownRom { sha1: "f72d89148dac39c696e30b10859ebc859226637b", size: 524288, label: "Kickstart v2.05 (A600HD)" },
    KnownRom { sha1: "02843c4253bbd29aba535b0aa3bd9a85034ecde4", size: 524288, label: "Kickstart v2.05 (A600HD)" },
    KnownRom { sha1: "d82ebb59afc53540ddf2d7187ecf239b7ea91590", size: 524288, label: "Kickstart v2.04 (A3000)" },
    KnownRom { sha1: "70033828182fffc7ed106e5373a8b89dda76faa5", size: 524288, label: "Kickstart v3.0 (A1200)" },
    KnownRom { sha1: "f0b4e9e29e12218c2d5bd7020e4e785297d91fd7", size: 524288, label: "Kickstart v3.0 (A4000)" },
    KnownRom { sha1: "81c631dd096bbb31d2af90299c76b774db74076c", size: 524288, label: "Kickstart v3.1 (A4000)" },
    KnownRom { sha1: "3b7f1493b27e212830f989f26ca76c02049f09ca", size: 524288, label: "Kickstart v3.1 (A500,A600,A2000)" },
    KnownRom { sha1: "e21545723fe8374e91342617604f1b3d703094f1", size: 524288, label: "Kickstart v3.1 (A1200)" },
    KnownRom { sha1: "f8e210d72b4c4853e0c9b85d223ba20e3d1b36ee", size: 524288, label: "Kickstart v3.1 (A3000)" },
    KnownRom { sha1: "c3c481160866e60d085e436a24db3617ff60b5f9", size: 524288, label: "Kickstart v3.1 (A4000)(Cloanto)" },
    KnownRom { sha1: "5fe04842d04a489720f0f4bb0e46948199406f49", size: 524288, label: "Kickstart v3.1 (A4000)" },
    KnownRom { sha1: "b0ec8b84d6768321e01209f11e6248f2f5281a21", size: 524288, label: "Kickstart v3.1 (A4000T)" },
    KnownRom { sha1: "3cbfc9e1fe396360157bd161de74fc901abee7ec", size: 524288, label: "Kickstart v3.X (A4000)(Cloanto)" },
    KnownRom { sha1: "6355a9ed5dc840422f9b73308a91be0d0bb506bd", size: 524288, label: "Kickstart v3.1.4-1 (A1200)" },
    KnownRom { sha1: "1a34b25a25e260e38e879172850143c80b461a64", size: 524288, label: "Kickstart v3.1.4-1 (A3000)" },
    KnownRom { sha1: "bad0ae388442db02eaeb6a5363ef43eb4e308ae6", size: 524288, label: "Kickstart v3.1.4-1 (A4000)" },
    KnownRom { sha1: "938a60a1d2c0d411f64bb27e5af40258b8decbf3", size: 524288, label: "Kickstart v3.1.4-1 (A4000T)" },
    KnownRom { sha1: "8a2405087ce182225656dd0b93069c45743f9e34", size: 524288, label: "Kickstart v3.1.4-1 (A500)" },
    KnownRom { sha1: "ef36c4638ee45de6bb93701761216c958cd0d57b", size: 524288, label: "Kickstart v3.1.4-2 (A1200)" },
    KnownRom { sha1: "d73ae3a36f12bb49bbf6ba04a890ff7aac419015", size: 524288, label: "Kickstart v3.1.4-2 (A3000)" },
    KnownRom { sha1: "aab44cd651e6b6f81a3effd8e0ba6b37ab322f32", size: 524288, label: "Kickstart v3.1.4-2 (A4000)" },
    KnownRom { sha1: "cd73aefe9cbfc258e7966cd14a7b2f4647c9ba45", size: 524288, label: "Kickstart v3.1.4-2 (A4000T)" },
    KnownRom { sha1: "d81cd6f131040895843d9dfa95a45bc95edd9704", size: 524288, label: "Kickstart v3.1.4-2 (A500)" },
    KnownRom { sha1: "5b2982876fec2166673be447643881262c84090e", size: 524288, label: "Kickstart v3.2 (A1200)" },
    KnownRom { sha1: "7bc0e75622d7254e11ea708e5d149007866c93ab", size: 524288, label: "Kickstart v3.2 (A3000)" },
    KnownRom { sha1: "37a8aa0b83782d75ce0b4a80631ff3459e91dc63", size: 524288, label: "Kickstart v3.2 (A4000)" },
    KnownRom { sha1: "ede1748eb2cbb1e86ac5bf7dc9b97246a4a54358", size: 524288, label: "Kickstart v3.2 (A4000T)" },
    KnownRom { sha1: "b88e364daf23c9c9920e548b0d3d944e65b1031d", size: 524288, label: "Kickstart v3.2 (A500/A600/A2000/A1000/CDTV)" },
    KnownRom { sha1: "0984fc0df07dc0585db0923d580629f70fca420d", size: 524288, label: "Kickstart v3.2.1 (A1200)" },
    KnownRom { sha1: "eb93508f0bb0cde81d88bf22d05045565d66ef74", size: 524288, label: "Kickstart v3.2.1 (A3000)" },
    KnownRom { sha1: "7db3c3226acc0bfe548c788cf1bc7c4c8774d66f", size: 524288, label: "Kickstart v3.2.1 (A4000)" },
    KnownRom { sha1: "a75fcd349680cedeab06e496c6338394e81788d7", size: 524288, label: "Kickstart v3.2.1 (A4000T)" },
    KnownRom { sha1: "8f64ada68a7f128ba782e8dc9fa583344171590a", size: 524288, label: "Kickstart v3.2.1 (A500/A600/A2000/A1000/CDTV)" },
    KnownRom { sha1: "d9622547d84741b52c5472607f9aca75565cbba3", size: 524288, label: "Kickstart v3.2.2 (A1200)" },
    KnownRom { sha1: "443d98369aca3ba51ab2d4789d14fa9a26379c0b", size: 524288, label: "Kickstart v3.2.2 (A3000)" },
    KnownRom { sha1: "a3207bd5b5a3ca629010f3159ac793040fa813ef", size: 524288, label: "Kickstart v3.2.2 (A4000)" },
    KnownRom { sha1: "8e3fff2675cc1b97b4f072b63fd4de7093f7c627", size: 524288, label: "Kickstart v3.2.2 (A4000T)" },
    KnownRom { sha1: "7d5ebe686b69d59a863cc77a36b2cd60359a9ed2", size: 524288, label: "Kickstart v3.2.2 (A500/A600/A2000/A1000/CDTV)" },
    KnownRom { sha1: "7a9095f1107966f90267dc4cb3c1972efb4b78a8", size: 524288, label: "Kickstart v3.2 (Walker)" },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for (table, sizes) in [(MAC_ROMS, [524_288, 1_048_576]), (KICKSTARTS, [262_144, 524_288])] {
            for rom in table {
                assert!(rom.sha1.len() == 40 && rom.sha1.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)), "{}", rom.sha1);
                assert!(sizes.contains(&rom.size), "{}", rom.label);
                assert!(!rom.label.is_empty() && !rom.label.contains('\n'));
                assert!(seen.insert(rom.sha1), "duplicate {}", rom.sha1);
            }
        }
    }
}
