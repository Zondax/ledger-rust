use std::{env, path::PathBuf};

fn sdk_includes(target: &str) -> impl IntoIterator<Item = PathBuf> {
    [
        PathBuf::from("include"),
        PathBuf::from("target").join(target).join("include"),
        PathBuf::from("lib_ux").join("include"),
        PathBuf::from("lib_cxng").join("include"),
        PathBuf::from("lib_bagl").join("include"),
        PathBuf::from("lib_nbgl").join("include"),
        PathBuf::from(
            env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set in build script"),
        )
        .join("bindgen")
        .join("include"),
    ]
}

#[derive(Debug, Clone, Copy)]
enum Device {
    NanoS,
    NanoX,
    NanoS2,
    Stax,
}

impl Device {
    pub fn sdk_includes(&self) -> impl IntoIterator<Item = PathBuf> {
        match self {
            Device::NanoS => sdk_includes("nanos"),
            Device::NanoX => sdk_includes("nanox"),
            Device::NanoS2 => sdk_includes("nanos2"),
            Device::Stax => sdk_includes("stax"),
        }
    }

    pub fn input_header(&self) -> PathBuf {
        let base = PathBuf::from(
            env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set in build script"),
        )
        .join("bindgen");

        let device = match self {
            Device::NanoS => "wrapperS.h",
            Device::NanoX => "wrapperX.h",
            Device::NanoS2 => "wrapperSP.h",
            Device::Stax => "wrapperFS.h",
        };

        base.join(device)
    }

    pub fn device_flags(&self) -> impl IntoIterator<Item = &'static str> {
        match self {
            Device::NanoS | Device::NanoX => ["-mcpu=cortex-m0plus", " -mthumb"],
            Device::NanoS2 | Device::Stax => ["-mcpu=cortex-m35p+nodsp", "-mthumb"],
        }
    }

    pub fn target(&self) -> impl IntoIterator<Item = &'static str> {
        match self {
            Device::NanoX | Device::NanoS => ["-target", "armv6m-none-eabi"],
            Device::NanoS2 | Device::Stax => ["-target", "armv8m-none-eabi"],
        }
    }
}

fn detect_device() -> Option<Device> {
    match env::var("TARGET_NAME").ok()?.as_str() {
        "TARGET_NANOS" => Some(Device::NanoS),
        "TARGET_NANOX" => Some(Device::NanoX),
        "TARGET_NANOS2" => Some(Device::NanoS2),
        "TARGET_STAX" => Some(Device::Stax),
        _ => None,
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=BOLOS_SDK");
    println!("cargo:rerun-if-env-changed=TARGET_NAME");

    if let Some(v) = env::var_os("BOLOS_SDK") {
        if !v.is_empty() {
            let sdk_path = PathBuf::from(v);

            let device = detect_device().expect("invalid or unable to retrieve TARGET_NAME");
            match device {
                Device::NanoS => println!("cargo:rustc-cfg=nanos"),
                Device::NanoX => println!("cargo:rustc-cfg=nanox"),
                Device::NanoS2 => println!("cargo:rustc-cfg=nanosplus"),
                Device::Stax => println!("cargo:rustc-cfg=stax"),
            }
            println!("cargo:rustc-cfg=bolos_sdk");

            let output = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR set in build script"))
                .join("bindings.rs");

            // Try both Ubuntu and Debian paths for GCC
            let gcc_paths = [
                "/usr/lib/gcc/arm-none-eabi",
                "/usr/lib/gcc-cross/arm-none-eabi", // Debian specific path
                "/usr/arm-none-eabi",
                "/usr/lib/arm-none-eabi",
            ];

            // Find the GCC version directory
            let gcc_version = std::process::Command::new("arm-none-eabi-gcc")
                .arg("-dumpversion")
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .unwrap_or_else(|| "10.3.1".to_string());

            // Try to find the correct include path
            let mut found_gcc_include = None;
            for base_path in gcc_paths.iter() {
                let test_path = format!("{}/{}/include", base_path, gcc_version.trim());
                if std::path::Path::new(&test_path).exists() {
                    found_gcc_include = Some(test_path);
                    break;
                }
            }

            let gcc_include = found_gcc_include.unwrap_or_else(|| {
                println!("cargo:warning=Could not find GCC include directory, using default path");
                format!(
                    "/usr/lib/gcc-cross/arm-none-eabi/{}/include",
                    gcc_version.trim()
                )
            });

            let bindings = bindgen::builder()
                .use_core()
                .derive_default(true)
                .header(device.input_header().display().to_string())
                // Add GCC includes first
                .clang_arg(format!("-I{}", gcc_include))
                // Then ARM includes with Debian paths
                .clang_arg("-I/usr/arm-none-eabi/include")
                .clang_arg("-I/usr/include/arm-none-eabi") // Debian specific path
                .clang_arg("-I/usr/lib/arm-none-eabi/include")
                // Device specific flags
                .clang_args(device.target())
                .clang_args(device.device_flags())
                // SDK includes
                .clang_args(
                    device
                        .sdk_includes()
                        .into_iter()
                        .map(|inc| sdk_path.join(inc))
                        .map(|path| format!("-I{}", path.display())),
                )
                .clang_arg(format!("-I{}", sdk_path.display()))
                .clang_arg(format!("-I{}/include", sdk_path.display()))
                .clang_arg(format!("-I{}/io/include", sdk_path.display()))
                .clang_arg(format!("-I{}/io_legacy/include", sdk_path.display()))
                .clang_arg("-D OS_IO_SEPH_BUFFER_SIZE=272")
                .generate()
                .expect("able to generate bindings");
            bindings
                .write_to_file(output)
                .expect("writing bindings to file");
        } else {
            panic!("BOLOS_SDK is not valid");
        }
    } else {
        println!("cargo:warning=BOLOS_SDK not set, not exporting anything")
    }
}
