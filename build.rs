fn main() {
    // Ensure the static-react/dist directory exists so that RustEmbed doesn't panic
    let path = std::path::Path::new("src/datafold_node/static-react/dist");
    if !path.exists() {
        // Create the directory if it doesn't exist to satisfy the macro
        // The actual content will be populated by the frontend build, but
        // for the rust build to succeed, the folder must exist.
        std::fs::create_dir_all(path).expect("Failed to create static assets directory");

        // Create an empty index.html so there's at least one file
        // This prevents "folder is empty" errors if RustEmbed is picky
        std::fs::write(
            path.join("index.html"),
            "<!DOCTYPE html><html><body>Placeholder</body></html>",
        )
        .expect("Failed to create placeholder index.html");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
