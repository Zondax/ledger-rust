fn main() {
    println!("cargo:rerun-if-env-changed=BOLOS_SDK");
    println!("cargo::rustc-check-cfg=cfg(bolos_sdk)");
    println!("cargo::rustc-check-cfg=cfg(__impl)");
    println!("cargo::rustc-check-cfg=cfg(__mock)");

    if let Some(v) = std::env::var_os("BOLOS_SDK") {
        if !v.is_empty() {
            println!("cargo:rustc-cfg=bolos_sdk");
            println!("cargo:rustc-cfg=__impl");
        } else {
            println!("cargo:rustc-cfg=__mock");
        }
    } else {
        println!("cargo:rustc-cfg=__mock");
    }
}
