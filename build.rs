fn main() {
    println!("cargo:rerun-if-env-changed=GXI_LOCALE_DIR");
    println!("cargo:rerun-if-env-changed=GXI_APP_ID");
    println!("cargo:rerun-if-env-changed=GXI_VERSION");
    println!("cargo:rerun-if-env-changed=GXI_PLUGIN_DIR");
    println!("cargo:rerun-if-env-changed=GXI_NAME");
}
