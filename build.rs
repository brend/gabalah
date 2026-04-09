use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/icons/gabalah.ico");

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();

    if target_os == "windows" && target_env == "msvc" {
        if let Err(err) = embed_windows_icon() {
            println!("cargo:warning=Failed to embed Windows icon: {err}");
        }
    }
}

fn embed_windows_icon() -> Result<(), Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let icon_path = manifest_dir
        .join("assets")
        .join("icons")
        .join("gabalah.ico");
    let rc_path = out_dir.join("gabalah-icon.rc");
    let res_path = out_dir.join("gabalah-icon.res");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_else(|_| "x86_64".to_string());
    let rc_exe = find_rc_exe(&target_arch)?;

    let icon_path_literal = icon_path.to_string_lossy().replace('\\', "\\\\");
    fs::write(&rc_path, format!("1 ICON \"{icon_path_literal}\"\n"))?;

    let status = Command::new(rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc_path)
        .status()?;

    if !status.success() {
        return Err(format!("rc.exe exited with status {status}").into());
    }

    println!("cargo:rustc-link-arg-bin=gabalah={}", res_path.display());
    Ok(())
}

fn find_rc_exe(target_arch: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = env::var("RC") {
        let candidate = PathBuf::from(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    let arch_dir = match target_arch {
        "x86_64" => "x64",
        "x86" => "x86",
        "aarch64" => "arm64",
        _ => "x64",
    };

    let kits_root = env::var_os("ProgramFiles(x86)")
        .or_else(|| env::var_os("ProgramFiles"))
        .map(PathBuf::from)
        .ok_or("Program Files directory is not available")?;
    let bin_root = kits_root.join("Windows Kits").join("10").join("bin");

    let mut candidates = Vec::new();
    for entry in fs::read_dir(&bin_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let rc_path = entry.path().join(arch_dir).join("rc.exe");
        if rc_path.is_file() {
            candidates.push(rc_path);
        }
    }

    candidates.sort_by_key(|candidate| version_key(candidate));
    candidates
        .pop()
        .ok_or_else(|| format!("Could not find rc.exe under {}", bin_root.display()).into())
}

fn version_key(path: &Path) -> Vec<u32> {
    let version = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default();

    version
        .split('.')
        .map(|segment| segment.parse::<u32>().unwrap_or_default())
        .collect()
}
