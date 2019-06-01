use std::process::Command;

fn main() {
    // Compile Gresource
    Command::new("glib-compile-resources")
        .args(&["--generate", "resources.xml"])
        .current_dir("src/ui")
        .status()
        .unwrap();
}
