use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Tell cargo to rerun this build script if Python environment changes
    println!("cargo:rerun-if-env-changed=PYO3_PYTHON");
    println!("cargo:rerun-if-env-changed=PYTHON_SYS_EXECUTABLE");

    // Respect PyO3's Python executable selection
    let python_exe = find_python_executable();

    // Try python-config first, then fall back to sysconfig
    let mut config_succeeded = false;
    if let Some(python_config) = find_python_config(&python_exe) {
        config_succeeded = configure_linking_with_config(&python_config);
    }

    if !config_succeeded {
        // Fallback: use sysconfig directly
        configure_linking_with_sysconfig(&python_exe);
    }
}

/// Find the Python executable that PyO3 will use
fn find_python_executable() -> PathBuf {
    // Check PyO3 environment variables in order of precedence
    if let Ok(python_path) = env::var("PYO3_PYTHON") {
        return PathBuf::from(python_path);
    }

    if let Ok(python_path) = env::var("PYTHON_SYS_EXECUTABLE") {
        return PathBuf::from(python_path);
    }

    // Fallback to python3 on PATH
    PathBuf::from("python3")
}

/// Find the python-config executable corresponding to the Python interpreter
fn find_python_config(python_exe: &PathBuf) -> Option<PathBuf> {
    // Try to derive python-config from python executable path
    if let Some(parent) = python_exe.parent() {
        // Try python3-config first
        let config_path = parent.join("python3-config");
        if config_path.exists() {
            return Some(config_path);
        }

        // Try python-config
        let config_path = parent.join("python-config");
        if config_path.exists() {
            return Some(config_path);
        }

        // Try versioned python-config (e.g., python3.12-config)
        if let Ok(version_output) = Command::new(python_exe)
            .arg("-c")
            .arg("import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
            .output()
        {
            if version_output.status.success() {
                let version = String::from_utf8_lossy(&version_output.stdout)
                    .trim()
                    .to_string();
                let versioned_config = parent.join(format!("python{}-config", version));
                if versioned_config.exists() {
                    return Some(versioned_config);
                }
            }
        }
    }

    // Fallback: try python-config executables on PATH when no parent or derived paths don't exist
    // This handles cases like python_exe = "python3" with no parent directory

    // Try python3-config on PATH
    if let Ok(output) = Command::new("python3-config").arg("--help").output() {
        if output.status.success() {
            return Some(PathBuf::from("python3-config"));
        }
    }

    // Try python-config on PATH
    if let Ok(output) = Command::new("python-config").arg("--help").output() {
        if output.status.success() {
            return Some(PathBuf::from("python-config"));
        }
    }

    // Try versioned python-config on PATH
    if let Ok(version_output) = Command::new(python_exe)
        .arg("-c")
        .arg("import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
        .output()
    {
        if version_output.status.success() {
            let version = String::from_utf8_lossy(&version_output.stdout)
                .trim()
                .to_string();
            let versioned_config = format!("python{}-config", version);
            if let Ok(output) = Command::new(&versioned_config).arg("--help").output() {
                if output.status.success() {
                    return Some(PathBuf::from(versioned_config));
                }
            }
        }
    }

    None
}

/// Configure linking using python-config
/// Returns true if successful, false if caller should use fallback
fn configure_linking_with_config(python_config: &PathBuf) -> bool {
    // Try --embed first (Python 3.8+), which includes -lpython
    let args = if let Ok(output) = Command::new(python_config)
        .arg("--embed")
        .arg("--ldflags")
        .output()
    {
        if output.status.success() {
            vec!["--embed", "--ldflags"]
        } else {
            vec!["--ldflags"]
        }
    } else {
        vec!["--ldflags"]
    };

    if let Ok(output) = Command::new(python_config).args(&args).output() {
        if !output.status.success() {
            println!(
                "cargo:warning=python-config --ldflags failed with exit code {:?}, falling back to sysconfig",
                output.status.code()
            );
            // Don't parse potentially incomplete output, let caller handle fallback
            return false;
        }

        let ldflags = String::from_utf8_lossy(&output.stdout);

        // Also add Python library directory for cases where python-config doesn't include it
        // This is particularly important for pyenv installations
        if let Ok(libdir_output) = Command::new(python_config).arg("--prefix").output() {
            if libdir_output.status.success() {
                let prefix = String::from_utf8_lossy(&libdir_output.stdout)
                    .trim()
                    .to_string();
                let lib_path = format!("{}/lib", prefix);
                println!("cargo:rustc-link-search=native={}", lib_path);
            }
        }

        emit_linker_flags(ldflags.split_whitespace().collect());

        return true;
    }

    // Failed to execute python-config
    false
}

/// Configure linking using Python's sysconfig module
fn configure_linking_with_sysconfig(python_exe: &PathBuf) {
    let script = r"
import sysconfig
import sys
import os

# Get library directory
lib_dir = sysconfig.get_config_var('LIBDIR')
if lib_dir:
    print(f'LIBDIR:{lib_dir}')

# Get library name
lib_name = sysconfig.get_config_var('LDLIBRARY')
if not lib_name:
    # Fallback to LIBRARY
    lib_name = sysconfig.get_config_var('LIBRARY')
if lib_name:
    # Remove lib prefix and .a/.so suffix
    if lib_name.startswith('lib'):
        lib_name = lib_name[3:]
    if lib_name.endswith('.a'):
        lib_name = lib_name[:-2]
    elif lib_name.endswith('.so'):
        lib_name = lib_name[:-3]
    elif lib_name.endswith('.dylib'):
        lib_name = lib_name[:-6]
    print(f'LIBRARY:{lib_name}')

# Get additional linker flags
ldflags = sysconfig.get_config_var('LDFLAGS')
if ldflags:
    print(f'LDFLAGS:{ldflags}')

# Get additional libs used by python-config
for key in ('LIBS', 'SYSLIBS', 'LINKFORSHARED'):
    val = sysconfig.get_config_var(key)
    if val:
        print(f'{key}:{val}')

# Get framework directory on macOS
if sys.platform == 'darwin':
    framework_dir = sysconfig.get_config_var('PYTHONFRAMEWORKDIR')
    if framework_dir:
        print(f'FRAMEWORKDIR:{framework_dir}')
    framework = sysconfig.get_config_var('PYTHONFRAMEWORK')
    if framework:
        print(f'FRAMEWORK:{framework}')
";

    if let Ok(output) = Command::new(python_exe).arg("-c").arg(script).output() {
        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if let Some(lib_dir) = line.strip_prefix("LIBDIR:") {
                println!("cargo:rustc-link-search=native={}", lib_dir);
            } else if let Some(lib_name) = line.strip_prefix("LIBRARY:") {
                println!("cargo:rustc-link-lib={}", lib_name);
            } else if let Some(ldflags) = line.strip_prefix("LDFLAGS:") {
                // Parse additional LDFLAGS (including framework flags that can appear here on macOS)
                emit_linker_flags(ldflags.split_whitespace().collect());
            } else if let Some(extra) = line
                .strip_prefix("LIBS:")
                .or_else(|| line.strip_prefix("SYSLIBS:"))
                .or_else(|| line.strip_prefix("LINKFORSHARED:"))
            {
                emit_linker_flags(extra.split_whitespace().collect());
            } else if let Some(framework_dir) = line.strip_prefix("FRAMEWORKDIR:") {
                println!("cargo:rustc-link-search=framework={}", framework_dir);
            } else if let Some(framework) = line.strip_prefix("FRAMEWORK:") {
                println!("cargo:rustc-link-lib=framework={}", framework);
            }
        }
    }
}

fn emit_linker_flags(flags: Vec<&str>) {
    let mut skip_next = false;
    for (i, flag) in flags.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }

        if let Some(path) = flag.strip_prefix("-L") {
            if !path.is_empty() {
                println!("cargo:rustc-link-search=native={}", path);
            } else if i + 1 < flags.len() {
                println!("cargo:rustc-link-search=native={}", flags[i + 1]);
                skip_next = true;
            }
        } else if let Some(lib) = flag.strip_prefix("-l") {
            let lib = if !lib.is_empty() {
                lib
            } else if i + 1 < flags.len() {
                skip_next = true;
                flags[i + 1]
            } else {
                ""
            };
            // Only skip system libraries that cargo handles automatically
            if !lib.is_empty() && !["intl", "dl", "util", "rt"].contains(&lib) {
                println!("cargo:rustc-link-lib={}", lib);
            }
        } else if let Some(framework) = flag.strip_prefix("-framework") {
            if !framework.is_empty() {
                // -frameworkName
                println!("cargo:rustc-link-lib=framework={}", framework);
            } else if i + 1 < flags.len() {
                // -framework Name
                let framework = flags[i + 1];
                println!("cargo:rustc-link-lib=framework={}", framework);
                skip_next = true;
            }
        } else if *flag == "-F" && i + 1 < flags.len() {
            // -F <path> (space-separated)
            let path = flags[i + 1];
            println!("cargo:rustc-link-search=framework={}", path);
            skip_next = true;
        } else if let Some(path) = flag.strip_prefix("-F") {
            println!("cargo:rustc-link-search=framework={}", path);
        } else if flag.starts_with("-Wl,") {
            // Pass through linker flags like -Wl,-rpath,path
            println!("cargo:rustc-link-arg={}", flag);
        } else if *flag == "-pthread" {
            println!("cargo:rustc-link-arg=-pthread");
        } else if flag.starts_with('-') {
            // Preserve any other linker flags we don't explicitly parse
            println!("cargo:rustc-link-arg={}", flag);
        }
    }
}
