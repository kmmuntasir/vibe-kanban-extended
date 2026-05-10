use std::{fs, path::Path};

fn main() {
    let dist_path = Path::new("../../packages/local-web/dist");
    if !dist_path.exists() {
        fs::create_dir_all(dist_path).unwrap();

        let dummy_html = r#"<!DOCTYPE html>
<html><head><title>Build web app first</title></head>
<body><h1>Please build @vibe/local-web first</h1></body></html>"#;

        fs::write(dist_path.join("index.html"), dummy_html).unwrap();
    }
}
