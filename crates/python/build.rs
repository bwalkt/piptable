use std::process::Command;

fn main() {
    // Get Python library path and name for linking
    if let Ok(output) = Command::new("python3-config").arg("--ldflags").output() {
        let ldflags = String::from_utf8_lossy(&output.stdout);

        // Parse -L and -l flags from python3-config output
        for flag in ldflags.split_whitespace() {
            if flag.starts_with("-L") {
                let path = &flag[2..];
                println!("cargo:rustc-link-search=native={}", path);
            } else if flag.starts_with("-l")
                && !flag.starts_with("-lintl")
                && !flag.starts_with("-ldl")
            {
                let lib = &flag[2..];
                println!("cargo:rustc-link-lib={}", lib);
            }
        }
    }

    // Additional approach: try to find and link Python library directly
    if let Ok(output) = Command::new("python3")
        .arg("-c")
        .arg("import sys; print(sys.executable)")
        .output()
    {
        let python_path = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Extract version
        if let Ok(version_output) = Command::new("python3")
            .arg("-c")
            .arg("import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
            .output()
        {
            let version = String::from_utf8_lossy(&version_output.stdout)
                .trim()
                .to_string();

            // For pyenv installations, link the specific library
            if python_path.contains(".pyenv") {
                let lib_path = python_path.replace("/bin/python3", "/lib");
                println!("cargo:rustc-link-search=native={}", lib_path);
                println!("cargo:rustc-link-lib=python{}", version);
            }
        }
    }
}
