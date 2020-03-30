use std::env;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.so";
#[cfg(target_os = "linux")]
const EXPECTED_JLI_FILENAME: &str = "libjli.so";

#[cfg(target_os = "macos")]
const EXPECTED_JVM_FILENAME: &str = "libjvm.dylib";
#[cfg(target_os = "macos")]
const EXPECTED_JLI_FILENAME: &str = "libjli.dylib";

fn main() {
    if cfg!(windows) {
        panic!("windows not supported");
    }

    let java_home = env::var("JAVA_HOME").expect("no JAVA_HOME set in env");

    let libjvm_path = find_lib(&java_home, EXPECTED_JVM_FILENAME).expect("failed to find libjvm");
    let libjli_path = find_lib(&java_home, EXPECTED_JLI_FILENAME).expect("failed to find libjli");

    println!("cargo:rustc-link-search=native={}", libjli_path.display());
    println!("cargo:rustc-link-search=native={}", libjvm_path.display());

    println!("cargo:rerun-if-env-changed=JAVA_HOME");
    println!("cargo:rustc-link-lib=dylib=jli");
    println!("cargo:rustc-link-lib=dylib=jvm");
}

fn find_lib<S: AsRef<Path>>(path: S, expected_file_name: &str) -> Option<PathBuf> {
    let walker = walkdir::WalkDir::new(path).follow_links(true);
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let file_name = entry.file_name().to_str().unwrap_or("");
        if file_name == expected_file_name {
            return entry.path().parent().map(Into::into);
        }
    }

    None
}
