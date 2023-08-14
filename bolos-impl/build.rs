use std::env;

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
    println!("cargo:rerun-if-env-changed=TARGET_NAME");
    println!("cargo:rerun-if-env-changed=BOLOS_SDK");

    if let Some(v) = env::var_os("BOLOS_SDK") {
        if !v.is_empty() {
            match detect_device().expect("invalid or unable to retrieve TARGET_NAME") {
                Device::NanoS => println!("cargo:rustc-cfg=nanos"),
                Device::NanoX => println!("cargo:rustc-cfg=nanox"),
                Device::NanoS2 => println!("cargo:rustc-cfg=nanosplus"),
                Device::Stax => println!("cargo:rustc-cfg=stax"),
            }
            println!("cargo:rustc-cfg=bolos_sdk");
        } else {
            panic!("empty BOLOS_SDK is not valid");
        }
    } else {
        println!("cargo:warning=[IMPL] BOLOS_SDK not set, not exporting anything")
    }
}
