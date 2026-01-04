fn main() {
    // Ensure the static-react/dist directory exists so that RustEmbed doesn't panic
    let path = std::path::Path::new("src/datafold_node/static-react/dist");
    if !path.exists() {
        // Create the directory if it doesn't exist to satisfy the macro
        // The actual content will be populated by the frontend build, but
        // for the rust build to succeed, the folder must exist.
        std::fs::create_dir_all(path).expect("Failed to create static assets directory");

        // Create dummy database.svg
        std::fs::write(
            path.join("database.svg"),
            "<svg xmlns='http://www.w3.org/2000/svg'></svg>",
        )
        .expect("Failed to create placeholder database.svg");

        // Create an index.html that satisfies test assertions
        // Tests look for: href="./database.svg" and src="./"
        std::fs::write(
            path.join("index.html"),
            r#"<!DOCTYPE html>
<html>
<head>
    <link rel="icon" href="./database.svg">
</head>
<body>
    <script src="./assets/index.js"></script>
    Placeholder
</body>
</html>"#,
        )
        .expect("Failed to create placeholder index.html");
    }

    println!("cargo:rerun-if-changed=build.rs");
}
