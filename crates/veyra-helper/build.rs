fn main() {
    if std::env::var_os("CARGO_FEATURE_P0_05_PROTOTYPE").is_none() {
        return;
    }
    assert_eq!(std::env::var("CARGO_CFG_TARGET_OS").unwrap(), "macos");
    assert_eq!(
        std::env::var("MACOSX_DEPLOYMENT_TARGET").as_deref(),
        Ok("15.0")
    );
    cc::Build::new()
        .file("native/network.m")
        .flag("-fobjc-arc")
        .compile("p005_network");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=SystemConfiguration");
    println!("cargo:rerun-if-changed=native/network.m");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
}
