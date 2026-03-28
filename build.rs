use std::process::Command;

fn main() {
    // Re-run if Python target changes
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");

    // Determine which Python to query (respect PYO3_PYTHON)
    let python = std::env::var("PYO3_PYTHON").unwrap_or_else(|_| "python3".to_string());

    // Get Python's LIBDIR so we can embed an rpath for libpython
    let output = Command::new(&python)
        .args(["-c", "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"])
        .output();

    let libdir = match output {
        Ok(out) if out.status.success() => {
            let dir = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if dir.is_empty() || dir == "None" {
                println!("cargo:warning=Python LIBDIR is empty or None, skipping rpath");
                return;
            }
            dir
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            println!("cargo:warning=Failed to get Python LIBDIR: {stderr}");
            return;
        }
        Err(e) => {
            println!("cargo:warning=Could not run {python}: {e}");
            return;
        }
    };

    // Emit rpath so the binary finds libpython at runtime without DYLD_LIBRARY_PATH
    println!("cargo:rustc-link-arg=-Wl,-rpath,{libdir}");

    // On macOS, also handle framework builds where libpython may be in a Frameworks dir
    #[cfg(target_os = "macos")]
    {
        let framework_output = Command::new(&python)
            .args([
                "-c",
                "import sysconfig; v = sysconfig.get_config_var('PYTHONFRAMEWORKPREFIX'); print(v if v else '')",
            ])
            .output();

        if let Ok(out) = framework_output {
            if out.status.success() {
                let framework_prefix = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !framework_prefix.is_empty() {
                    let framework_lib = format!("{framework_prefix}/lib");
                    if framework_lib != libdir {
                        println!("cargo:rustc-link-arg=-Wl,-rpath,{framework_lib}");
                    }
                }
            }
        }
    }
}
