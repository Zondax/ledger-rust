use std::{env, fs, path};

#[derive(Debug, Clone, Copy)]
enum Device {
    NanoS,
    NanoX,
    NanoS2,
    Stax,
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
            if let Ok(contents) = fs::read_to_string(path::Path::new(&v).join("Makefile.defines")) {
                if contents.contains("REVAMPED_IO") {
                    println!("cargo:rustc-cfg=revamped_io");
                }
            }

            match detect_device().expect("invalid or unable to retrieve TARGET_NAME") {
                Device::NanoS => println!("cargo:rustc-cfg=nanos"),
                Device::NanoX => println!("cargo:rustc-cfg=nanox"),
                Device::NanoS2 => println!("cargo:rustc-cfg=nanosplus"),
                Device::Stax => println!("cargo:rustc-cfg=stax"),
            }
            println!("cargo:rustc-cfg=bolos_sdk");
            println!("cargo:rustc-cfg=zemu_sdk");

            if let Some(log) = env::var_os("ZEMU_LOGGING") {
                if !log.is_empty() {
                    println!("cargo:rustc-cfg=zemu_logging");
                }
            }
        } else {
            panic!("BOLOS_SDK is not valid");
        }
    } else {
        println!("cargo:warning=BOLOS_SDK not set, not exporting anything")
    }
}
