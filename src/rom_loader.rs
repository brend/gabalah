#[cfg(feature = "rom-gzip")]
use flate2::read::GzDecoder;
#[cfg(feature = "rom-7z")]
use sevenz_rust::{Password, SevenZReader};
use std::fmt;
use std::fs;
use std::io;
#[cfg(any(feature = "rom-zip", feature = "rom-gzip", feature = "rom-7z"))]
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
#[cfg(feature = "rom-zip")]
use zip::ZipArchive;

const MAX_ROM_SIZE: usize = 32 * 1024;
const ZIP_MAGIC: &[u8; 4] = b"PK\x03\x04";
const GZIP_MAGIC: &[u8; 2] = &[0x1F, 0x8B];
const SEVEN_Z_MAGIC: &[u8; 6] = &[b'7', b'z', 0xBC, 0xAF, 0x27, 0x1C];

#[derive(Debug)]
pub enum RomLoadError {
    Io {
        path: PathBuf,
        source: io::Error,
    },
    InvalidCliArgument(String),
    ArchiveRead {
        path: PathBuf,
        format: &'static str,
        detail: String,
    },
    NoRomEntries {
        path: PathBuf,
        entries: Vec<String>,
    },
    AmbiguousRomEntries {
        path: PathBuf,
        entries: Vec<String>,
    },
    EntryNotFound {
        path: PathBuf,
        requested: String,
        entries: Vec<String>,
    },
    EntrySelectionUnsupported {
        path: PathBuf,
        format: &'static str,
    },
    FormatDisabled {
        path: PathBuf,
        format: &'static str,
        feature: &'static str,
    },
    RomTooLarge {
        path: PathBuf,
        source: String,
        size: usize,
        max_size: usize,
    },
}

impl fmt::Display for RomLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RomLoadError::Io { path, source } => {
                write!(
                    f,
                    "failed to read ROM input '{}': {source}",
                    path.to_string_lossy()
                )
            }
            RomLoadError::InvalidCliArgument(message) => f.write_str(message),
            RomLoadError::ArchiveRead {
                path,
                format,
                detail,
            } => write!(
                f,
                "failed to decode {format} archive '{}': {detail}",
                path.to_string_lossy()
            ),
            RomLoadError::NoRomEntries { path, entries } => write!(
                f,
                "archive '{}' has no .gb/.gbc entries (available: {})",
                path.to_string_lossy(),
                format_entries(entries)
            ),
            RomLoadError::AmbiguousRomEntries { path, entries } => write!(
                f,
                "archive '{}' has multiple .gb/.gbc entries ({}); pass --entry <archive-path>",
                path.to_string_lossy(),
                format_entries(entries)
            ),
            RomLoadError::EntryNotFound {
                path,
                requested,
                entries,
            } => write!(
                f,
                "entry '{}' was not found in archive '{}' (available: {})",
                requested,
                path.to_string_lossy(),
                format_entries(entries)
            ),
            RomLoadError::EntrySelectionUnsupported { path, format } => write!(
                f,
                "--entry is not supported for {format} input '{}'",
                path.to_string_lossy()
            ),
            RomLoadError::FormatDisabled {
                path,
                format,
                feature,
            } => write!(
                f,
                "{format} support is disabled for input '{}'; rebuild with `--features {feature}`",
                path.to_string_lossy()
            ),
            RomLoadError::RomTooLarge {
                path,
                source,
                size,
                max_size,
            } => write!(
                f,
                "{source} in '{}' is {size} bytes; maximum supported ROM size is {max_size} bytes",
                path.to_string_lossy()
            ),
        }
    }
}

impl std::error::Error for RomLoadError {}

pub fn load_rom_from_path(path: &Path, entry: Option<&str>) -> Result<Vec<u8>, RomLoadError> {
    let bytes = fs::read(path).map_err(|source| RomLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;

    let format = detect_format(path, &bytes);
    let (rom, source) = match format {
        RomFormat::Raw => (bytes, "ROM file".to_string()),
        RomFormat::Zip => load_rom_from_zip_or_err(path, &bytes, entry)?,
        RomFormat::Gzip => load_rom_from_gzip_or_err(path, &bytes, entry)?,
        RomFormat::SevenZip => load_rom_from_7z_or_err(path, &bytes, entry)?,
    };

    validate_rom_size(path, &source, rom.len())?;
    Ok(rom)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RomFormat {
    Raw,
    Zip,
    Gzip,
    SevenZip,
}

fn detect_format(path: &Path, bytes: &[u8]) -> RomFormat {
    if has_prefix(bytes, ZIP_MAGIC) {
        return RomFormat::Zip;
    }
    if has_prefix(bytes, GZIP_MAGIC) {
        return RomFormat::Gzip;
    }
    if has_prefix(bytes, SEVEN_Z_MAGIC) {
        return RomFormat::SevenZip;
    }

    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
    {
        Some(ext) if ext == "zip" => RomFormat::Zip,
        Some(ext) if ext == "gz" => RomFormat::Gzip,
        Some(ext) if ext == "7z" => RomFormat::SevenZip,
        _ => RomFormat::Raw,
    }
}

fn has_prefix(bytes: &[u8], prefix: &[u8]) -> bool {
    bytes.len() >= prefix.len() && &bytes[..prefix.len()] == prefix
}

fn load_rom_from_zip_or_err(
    path: &Path,
    bytes: &[u8],
    entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    #[cfg(feature = "rom-zip")]
    {
        return load_rom_from_zip(path, bytes, entry);
    }
    #[cfg(not(feature = "rom-zip"))]
    {
        let _ = (bytes, entry);
        Err(RomLoadError::FormatDisabled {
            path: path.to_path_buf(),
            format: "zip",
            feature: "rom-zip",
        })
    }
}

fn load_rom_from_gzip_or_err(
    path: &Path,
    bytes: &[u8],
    entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    #[cfg(feature = "rom-gzip")]
    {
        return load_rom_from_gzip(path, bytes, entry);
    }
    #[cfg(not(feature = "rom-gzip"))]
    {
        let _ = (bytes, entry);
        Err(RomLoadError::FormatDisabled {
            path: path.to_path_buf(),
            format: "gzip",
            feature: "rom-gzip",
        })
    }
}

fn load_rom_from_7z_or_err(
    path: &Path,
    bytes: &[u8],
    entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    #[cfg(feature = "rom-7z")]
    {
        return load_rom_from_7z(path, bytes, entry);
    }
    #[cfg(not(feature = "rom-7z"))]
    {
        let _ = (bytes, entry);
        Err(RomLoadError::FormatDisabled {
            path: path.to_path_buf(),
            format: "7z",
            feature: "rom-7z",
        })
    }
}

#[cfg(feature = "rom-zip")]
fn load_rom_from_zip(
    archive_path: &Path,
    bytes: &[u8],
    requested_entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    let reader = Cursor::new(bytes);
    let mut archive = ZipArchive::new(reader).map_err(|err| RomLoadError::ArchiveRead {
        path: archive_path.to_path_buf(),
        format: "zip",
        detail: err.to_string(),
    })?;

    let mut file_entries = Vec::new();
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|err| RomLoadError::ArchiveRead {
                path: archive_path.to_path_buf(),
                format: "zip",
                detail: err.to_string(),
            })?;
        if !entry.name().ends_with('/') {
            file_entries.push(entry.name().to_string());
        }
    }

    let selected = select_archive_entry(archive_path, requested_entry, &file_entries)?;
    let mut selected_entry =
        archive
            .by_name(&selected)
            .map_err(|err| RomLoadError::ArchiveRead {
                path: archive_path.to_path_buf(),
                format: "zip",
                detail: err.to_string(),
            })?;

    let mut rom = Vec::new();
    selected_entry
        .read_to_end(&mut rom)
        .map_err(|err| RomLoadError::ArchiveRead {
            path: archive_path.to_path_buf(),
            format: "zip",
            detail: err.to_string(),
        })?;

    Ok((rom, format!("entry '{selected}'")))
}

#[cfg(feature = "rom-gzip")]
fn load_rom_from_gzip(
    path: &Path,
    bytes: &[u8],
    requested_entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    if requested_entry.is_some() {
        return Err(RomLoadError::EntrySelectionUnsupported {
            path: path.to_path_buf(),
            format: "gzip",
        });
    }

    let mut decoder = GzDecoder::new(Cursor::new(bytes));
    let mut rom = Vec::new();
    decoder
        .read_to_end(&mut rom)
        .map_err(|err| RomLoadError::ArchiveRead {
            path: path.to_path_buf(),
            format: "gzip",
            detail: err.to_string(),
        })?;
    Ok((rom, "gzip stream".to_string()))
}

#[cfg(feature = "rom-7z")]
fn load_rom_from_7z(
    path: &Path,
    bytes: &[u8],
    requested_entry: Option<&str>,
) -> Result<(Vec<u8>, String), RomLoadError> {
    let mut reader = SevenZReader::new(
        Cursor::new(bytes.to_vec()),
        bytes.len() as u64,
        Password::empty(),
    )
    .map_err(|err| RomLoadError::ArchiveRead {
        path: path.to_path_buf(),
        format: "7z",
        detail: err.to_string(),
    })?;

    let file_entries: Vec<String> = reader
        .archive()
        .files
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.name.clone())
        .collect();

    let selected = select_archive_entry(path, requested_entry, &file_entries)?;
    let mut selected_rom = None;

    reader
        .for_each_entries(|entry, content| {
            if entry.is_directory() {
                return Ok(true);
            }

            if entry.name() == selected {
                let mut rom = Vec::new();
                content
                    .read_to_end(&mut rom)
                    .map_err(sevenz_rust::Error::io)?;
                selected_rom = Some(rom);
                return Ok(false);
            }

            io::copy(content, &mut io::sink())
                .map(|_| true)
                .map_err(sevenz_rust::Error::io)
        })
        .map_err(|err| RomLoadError::ArchiveRead {
            path: path.to_path_buf(),
            format: "7z",
            detail: err.to_string(),
        })?;

    match selected_rom {
        Some(rom) => Ok((rom, format!("entry '{selected}'"))),
        None => Err(RomLoadError::EntryNotFound {
            path: path.to_path_buf(),
            requested: selected,
            entries: file_entries,
        }),
    }
}

#[cfg(any(feature = "rom-zip", feature = "rom-7z"))]
fn select_archive_entry(
    path: &Path,
    requested_entry: Option<&str>,
    entries: &[String],
) -> Result<String, RomLoadError> {
    if let Some(requested) = requested_entry {
        if entries.iter().any(|entry| entry == requested) {
            return Ok(requested.to_string());
        }
        return Err(RomLoadError::EntryNotFound {
            path: path.to_path_buf(),
            requested: requested.to_string(),
            entries: entries.to_vec(),
        });
    }

    let rom_entries: Vec<String> = entries
        .iter()
        .filter(|entry| is_rom_entry_name(entry))
        .cloned()
        .collect();

    match rom_entries.len() {
        1 => Ok(rom_entries[0].clone()),
        0 => Err(RomLoadError::NoRomEntries {
            path: path.to_path_buf(),
            entries: entries.to_vec(),
        }),
        _ => Err(RomLoadError::AmbiguousRomEntries {
            path: path.to_path_buf(),
            entries: rom_entries,
        }),
    }
}

#[cfg(any(feature = "rom-zip", feature = "rom-7z"))]
fn is_rom_entry_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.ends_with(".gb") || lower.ends_with(".gbc")
}

fn validate_rom_size(path: &Path, source: &str, size: usize) -> Result<(), RomLoadError> {
    if size > MAX_ROM_SIZE {
        return Err(RomLoadError::RomTooLarge {
            path: path.to_path_buf(),
            source: source.to_string(),
            size,
            max_size: MAX_ROM_SIZE,
        });
    }
    Ok(())
}

fn format_entries(entries: &[String]) -> String {
    if entries.is_empty() {
        return "none".to_string();
    }
    entries.join(", ")
}

#[cfg(all(test, feature = "rom-zip", feature = "rom-gzip", feature = "rom-7z"))]
mod tests {
    use super::*;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use sevenz_rust::compress_to_path;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};
    use zip::write::FileOptions;
    use zip::ZipWriter;

    fn fixture_rom() -> Vec<u8> {
        vec![0x00; 0x4000]
    }

    fn unique_test_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be monotonic")
            .as_nanos();
        for attempt in 0..1024u32 {
            let dir = std::env::temp_dir().join(format!(
                "gabalah-rom-loader-{}-{nanos}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&dir) {
                Ok(()) => return dir,
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => panic!("temp test directory should be created: {err}"),
            }
        }
        panic!("failed to allocate unique temp test directory");
    }

    fn write_raw_rom(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, fixture_rom()).expect("raw ROM should be written");
        path
    }

    fn write_zip(dir: &Path, name: &str, entries: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let file = File::create(&path).expect("zip file should be created");
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (entry_name, bytes) in entries {
            zip.start_file(*entry_name, options)
                .expect("zip entry should be created");
            zip.write_all(bytes)
                .expect("zip entry bytes should be written");
        }
        zip.finish().expect("zip writer should finish");
        path
    }

    fn write_gzip(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        let file = File::create(&path).expect("gzip file should be created");
        let mut encoder = GzEncoder::new(file, Compression::default());
        encoder
            .write_all(bytes)
            .expect("gzip payload should be written");
        encoder.finish().expect("gzip writer should finish");
        path
    }

    fn write_7z(dir: &Path, archive_name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let source_dir = dir.join("sevenz-source");
        fs::create_dir_all(&source_dir).expect("7z source dir should be created");
        for (name, bytes) in files {
            fs::write(source_dir.join(name), bytes).expect("7z source file should be written");
        }
        let archive_path = dir.join(archive_name);
        compress_to_path(&source_dir, &archive_path).expect("7z archive should be created");
        archive_path
    }

    fn write_bytes(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, bytes).expect("bytes should be written");
        path
    }

    #[test]
    fn loads_raw_rom() {
        let dir = unique_test_dir();
        let rom_path = write_raw_rom(&dir, "game.gb");

        let rom = load_rom_from_path(&rom_path, None).expect("raw ROM should load");

        assert_eq!(rom, fixture_rom());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_single_zip_rom_entry() {
        let dir = unique_test_dir();
        let rom = fixture_rom();
        let zip_path = write_zip(&dir, "single.zip", &[("game.gb", &rom)]);

        let loaded = load_rom_from_path(&zip_path, None).expect("zip ROM should load");

        assert_eq!(loaded, rom);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn zip_with_multiple_roms_requires_entry_or_errors() {
        let dir = unique_test_dir();
        let rom_a = fixture_rom();
        let mut rom_b = fixture_rom();
        rom_b[0] = 0x99;
        let zip_path = write_zip(&dir, "multi.zip", &[("a.gb", &rom_a), ("b.gbc", &rom_b)]);

        let err = load_rom_from_path(&zip_path, None).expect_err("ambiguous zip should fail");
        assert!(matches!(err, RomLoadError::AmbiguousRomEntries { .. }));

        let loaded = load_rom_from_path(&zip_path, Some("b.gbc"))
            .expect("explicit entry should resolve ambiguity");
        assert_eq!(loaded, rom_b);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_gzip_rom() {
        let dir = unique_test_dir();
        let rom = fixture_rom();
        let gzip_path = write_gzip(&dir, "game.gb.gz", &rom);

        let loaded = load_rom_from_path(&gzip_path, None).expect("gzip ROM should load");

        assert_eq!(loaded, rom);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn loads_single_7z_rom_entry() {
        let dir = unique_test_dir();
        let rom = fixture_rom();
        let sevenz_path = write_7z(&dir, "game.7z", &[("game.gb", &rom)]);

        let loaded = load_rom_from_path(&sevenz_path, None).expect("7z ROM should load");

        assert_eq!(loaded, rom);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn entry_not_found_returns_error() {
        let dir = unique_test_dir();
        let rom = fixture_rom();
        let zip_path = write_zip(&dir, "single.zip", &[("game.gb", &rom)]);

        let err = load_rom_from_path(&zip_path, Some("missing.gb"))
            .expect_err("missing entry should fail");
        assert!(matches!(err, RomLoadError::EntryNotFound { .. }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn archive_without_rom_entries_returns_error() {
        let dir = unique_test_dir();
        let zip_path = write_zip(&dir, "assets.zip", &[("readme.txt", b"hello world")]);

        let err = load_rom_from_path(&zip_path, None).expect_err("missing ROM entries should fail");
        assert!(matches!(err, RomLoadError::NoRomEntries { .. }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn raw_rom_ignores_entry_override() {
        let dir = unique_test_dir();
        let rom_path = write_raw_rom(&dir, "game.gb");

        let loaded = load_rom_from_path(&rom_path, Some("ignored.gb"))
            .expect("raw ROM should ignore --entry");

        assert_eq!(loaded, fixture_rom());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn magic_bytes_take_precedence_over_file_extension() {
        let dir = unique_test_dir();
        let mut rom = fixture_rom();
        rom[0] = 0x5A;
        let archive_with_gb_extension = write_zip(&dir, "archive.gb", &[("game.gb", &rom)]);

        let loaded = load_rom_from_path(&archive_with_gb_extension, None)
            .expect("zip magic should override .gb extension");

        assert_eq!(loaded, rom);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn gzip_rejects_entry_override() {
        let dir = unique_test_dir();
        let gzip_path = write_gzip(&dir, "game.gb.gz", &fixture_rom());

        let err = load_rom_from_path(&gzip_path, Some("game.gb"))
            .expect_err("gzip should reject --entry");

        assert!(matches!(
            err,
            RomLoadError::EntrySelectionUnsupported { .. }
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sevenz_with_multiple_roms_requires_entry_or_errors() {
        let dir = unique_test_dir();
        let rom_a = fixture_rom();
        let mut rom_b = fixture_rom();
        rom_b[7] = 0xCE;
        let sevenz_path = write_7z(&dir, "multi.7z", &[("a.gb", &rom_a), ("b.gbc", &rom_b)]);

        let err = load_rom_from_path(&sevenz_path, None).expect_err("ambiguous 7z should fail");
        assert!(matches!(err, RomLoadError::AmbiguousRomEntries { .. }));

        let loaded = load_rom_from_path(&sevenz_path, Some("b.gbc"))
            .expect("explicit 7z entry should resolve ambiguity");
        assert_eq!(loaded, rom_b);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sevenz_entry_not_found_returns_error() {
        let dir = unique_test_dir();
        let sevenz_path = write_7z(&dir, "single.7z", &[("game.gb", &fixture_rom())]);

        let err = load_rom_from_path(&sevenz_path, Some("missing.gb"))
            .expect_err("missing 7z entry should fail");

        assert!(matches!(err, RomLoadError::EntryNotFound { .. }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn sevenz_without_rom_entries_returns_error() {
        let dir = unique_test_dir();
        let sevenz_path = write_7z(&dir, "assets.7z", &[("readme.txt", b"hello")]);

        let err =
            load_rom_from_path(&sevenz_path, None).expect_err("7z without ROM entries should fail");

        assert!(matches!(err, RomLoadError::NoRomEntries { .. }));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn rom_size_limit_applies_to_all_formats() {
        let dir = unique_test_dir();
        let exact = vec![0u8; MAX_ROM_SIZE];
        let over = vec![0u8; MAX_ROM_SIZE + 1];

        let raw_exact = write_bytes(&dir, "exact.gb", &exact);
        assert!(load_rom_from_path(&raw_exact, None).is_ok());

        let raw_over = write_bytes(&dir, "over.gb", &over);
        assert!(matches!(
            load_rom_from_path(&raw_over, None),
            Err(RomLoadError::RomTooLarge { .. })
        ));

        let zip_exact = write_zip(&dir, "exact.zip", &[("game.gb", &exact)]);
        assert!(load_rom_from_path(&zip_exact, None).is_ok());

        let zip_over = write_zip(&dir, "over.zip", &[("game.gb", &over)]);
        assert!(matches!(
            load_rom_from_path(&zip_over, None),
            Err(RomLoadError::RomTooLarge { .. })
        ));

        let gzip_exact = write_gzip(&dir, "exact.gb.gz", &exact);
        assert!(load_rom_from_path(&gzip_exact, None).is_ok());

        let gzip_over = write_gzip(&dir, "over.gb.gz", &over);
        assert!(matches!(
            load_rom_from_path(&gzip_over, None),
            Err(RomLoadError::RomTooLarge { .. })
        ));

        let sevenz_exact = write_7z(&dir, "exact.7z", &[("game.gb", &exact)]);
        assert!(load_rom_from_path(&sevenz_exact, None).is_ok());

        let sevenz_over = write_7z(&dir, "over.7z", &[("game.gb", &over)]);
        assert!(matches!(
            load_rom_from_path(&sevenz_over, None),
            Err(RomLoadError::RomTooLarge { .. })
        ));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_archives_return_archive_read_error() {
        let dir = unique_test_dir();

        let corrupt_zip = write_bytes(&dir, "broken.zip", &[ZIP_MAGIC[0], ZIP_MAGIC[1], 0x00]);
        assert!(matches!(
            load_rom_from_path(&corrupt_zip, None),
            Err(RomLoadError::ArchiveRead { .. })
        ));

        let corrupt_gzip = write_bytes(&dir, "broken.gz", &[GZIP_MAGIC[0], GZIP_MAGIC[1], 0x00]);
        assert!(matches!(
            load_rom_from_path(&corrupt_gzip, None),
            Err(RomLoadError::ArchiveRead { .. })
        ));

        let mut sevenz_bytes = Vec::from(SEVEN_Z_MAGIC.as_ref());
        sevenz_bytes.extend_from_slice(&[0x00, 0x01, 0x02, 0x03]);
        let corrupt_sevenz = write_bytes(&dir, "broken.7z", &sevenz_bytes);
        assert!(matches!(
            load_rom_from_path(&corrupt_sevenz, None),
            Err(RomLoadError::ArchiveRead { .. })
        ));

        let _ = fs::remove_dir_all(dir);
    }
}
