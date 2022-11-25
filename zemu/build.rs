use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=BOLOS_SDK");

    if let Some(v) = env::var_os("BOLOS_SDK") {
        if !v.is_empty() {
            match env::var("TARGET_NAME")
                .expect("unable to get TARGET_NAME")
                .as_str()
            {
                "TARGET_NANOX" => println!("cargo:rustc-cfg=nanox"),
                "TARGET_NANOS" => println!("cargo:rustc-cfg=nanos"),
                "TARGET_NANOS2" => println!("cargo:rustc-cfg=nanosplus"),
                _ => panic!("TARGET_NAME is not valid"),
            }

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
