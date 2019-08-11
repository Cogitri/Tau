use std::fs::remove_file;
use std::path::Path;
use std::process::Command;
use std::str::from_utf8;

fn main() {
    // Remove old versions of the gresource to make sure we're using the latest version
    if Path::new("src/ui/resources.gresource").exists() {
        remove_file("src/ui/resources.gresource").unwrap();
    }

    // Compile Gresource
    let output =
        Command::new(option_env!("GRESOURCE_BINARY_PATH").unwrap_or("glib-compile-resources"))
            .args(&["--generate", "resources.xml"])
            .current_dir("src/ui")
            .output()
            .unwrap();

    if !output.status.success() {
        println!("Failed to generate GResources!");
        println!(
            "glib-compile-resources stdout: {}",
            from_utf8(&output.stdout).unwrap()
        );
        println!(
            "glib-compile-resources stderr: {}",
            from_utf8(&output.stderr).unwrap()
        );
        panic!("Can't continue build without GResources!");
    }

    println!("cargo:rerun-if-changed=src/ui/close_tab.glade");
    println!("cargo:rerun-if-changed=src/ui/ev.glade");
    println!("cargo:rerun-if-changed=src/ui/find_replace.glade");
    println!("cargo:rerun-if-changed=src/ui/resources.xml");
}
