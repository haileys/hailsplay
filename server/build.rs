use std::{process::Command, path::PathBuf, collections::HashMap};
use serde::Deserialize;

#[derive(Deserialize)]
struct ManifestEntry {
    file: String,
}

fn main() {
    // only build and bundle the frontend if the bundle feature is set
    if std::env::var("CARGO_FEATURE_BUNDLE").is_err() {
        return;
    }

    let bundle_out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());

    let status = Command::new("../script/build-frontend")
        .env("OUT_DIR", &bundle_out_dir)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    if !status.success() {
        panic!("script/build-frontend failed");
    }

    let manifest_path = bundle_out_dir.join("manifest.json");

    let manifest_json = std::fs::read_to_string(manifest_path)
        .expect("read manifest.json");

    let manifest: HashMap<String, ManifestEntry> = serde_json::from_str(&manifest_json)
        .expect("parse manifest.json");

    println!("cargo:rustc-env=BUNDLE_DIR={}", bundle_out_dir.display());

    println!("cargo:rustc-env=BUNDLE_INDEX_HTML_PATH={}",
        bundle_out_dir.join("index.html").display());

    let index_css = manifest.get("index.css").unwrap();
    println!("cargo:rustc-env=BUNDLE_INDEX_CSS_URL=/{}", &index_css.file);
    println!("cargo:rustc-env=BUNDLE_INDEX_CSS_PATH={}",
        bundle_out_dir.join(&index_css.file).display());

    // this is weird, the key is actually named index.html despite it
    // being the manifest entry for index.js
    let index_js = manifest.get("index.html").unwrap();
    println!("cargo:rustc-env=BUNDLE_INDEX_JS_URL=/{}", &index_js.file);
    println!("cargo:rustc-env=BUNDLE_INDEX_JS_PATH={}",
        bundle_out_dir.join(&index_js.file).display());
}
