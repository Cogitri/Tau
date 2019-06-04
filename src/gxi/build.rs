use std::process::Command;
use std::str::from_utf8;

fn main() {
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
}
