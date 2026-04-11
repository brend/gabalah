use std::fs;
use std::path::{Path, PathBuf};

fn collect_rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).expect("directory should be readable");
    for entry in entries {
        let path = entry.expect("entry should be readable").path();
        if path.is_dir() {
            collect_rust_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_direct_cpu_memory_field_access_outside_cpu_core() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for dir in ["src", "tests", "benches"] {
        collect_rust_files(&root.join(dir), &mut files);
    }

    let allowlist = [Path::new("src/cpu/core.rs")];

    for file in files {
        let relative = file
            .strip_prefix(root)
            .expect("path should be within project root");
        if allowlist.contains(&relative) || relative == Path::new("tests/architecture.rs") {
            continue;
        }

        let contents = fs::read_to_string(&file).expect("file should be readable");
        assert!(
            !contents.contains("cpu.memory."),
            "direct Cpu memory field access found in {}",
            relative.display()
        );
    }
}
