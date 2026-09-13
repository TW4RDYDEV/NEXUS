fn main() {
    println!("cargo:rerun-if-env-changed=PROFILE");
    let root = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .to_path_buf();
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let owns_repository = git(&["rev-parse", "--show-toplevel"])
        .and_then(|p| std::fs::canonicalize(p).ok())
        == std::fs::canonicalize(&root).ok();
    let commit = if owns_repository {
        git(&["rev-parse", "HEAD"]).filter(|c| {
            (c.len() == 40 || c.len() == 64) && c.bytes().all(|b| b.is_ascii_hexdigit())
        })
    } else {
        None
    };
    if owns_repository {
        if let Some(git_dir) = git(&["rev-parse", "--absolute-git-dir"]) {
            println!("cargo:rerun-if-changed={git_dir}/HEAD");
            println!("cargo:rerun-if-changed={git_dir}/refs");
            println!("cargo:rerun-if-changed={git_dir}/packed-refs");
        }
    }
    println!(
        "cargo:rustc-env=NEXUS_BUILD_COMMIT={}",
        commit.as_deref().unwrap_or("unavailable")
    );
    println!(
        "cargo:rustc-env=NEXUS_BUILD_MODE={}",
        std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into())
    );

    #[cfg(feature = "desktop")]
    {
        // GCC can inject a second Windows manifest at the end of the link.
        // Preserve Tauri's Common Controls v6 manifest and override only that
        // optional GCC endfile. Without v6, TaskDialogIndirect cannot load.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            let linker = std::env::var("CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER")
                .unwrap_or_else(|_| "gcc".into());
            if let Ok(output) = std::process::Command::new(linker)
                .arg("-dumpspecs")
                .output()
            {
                let specs = String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n");
                if let Some(endfile) = specs
                    .split("*endfile:\n")
                    .nth(1)
                    .and_then(|s| s.lines().next())
                {
                    let default = "%{!shared:%:if-exists(default-manifest.o%s)}";
                    if endfile.contains(default) {
                        let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
                            .join("nexus-manifest.specs");
                        std::fs::write(
                            &path,
                            format!(
                                "%rename endfile nexus_original_endfile\n*endfile:\n{}\n",
                                endfile.replace(default, "")
                            ),
                        )
                        .expect("Cannot write GCC manifest override");
                        println!("cargo:rustc-link-arg-bins=-specs={}", path.display());
                    }
                }
            }
        }
        tauri_build::build();
    }
}
