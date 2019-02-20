use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("plugin-dir.in");
    let mut f = File::create(&dest_path).unwrap();

    let plugin_dir_default = "/usr/local/lib/gxi/plugins";

    let plugin_dir = if let Some(plugin_dir_env) = env::var_os("GXI_PLUGIN_DIR") {
        plugin_dir_env.to_str().unwrap().to_string()
    } else {
        println!(
            "Didn't specify GXI_PLGUIN_DIR, defaulting to {}",
            plugin_dir_default
        );
        plugin_dir_default.to_string()
    };

    f.write_all(plugin_dir.as_bytes()).unwrap();
}
