use std::env;
use std::io::Write;
use std::fs::{self, File};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();

    for asset_result in fs::read_dir("assets").expect("Couldn't read assets folder") {
        let asset = asset_result.expect("Couldn't inspect asset file");
        let asset_path = asset.path();
        let asset_filename_component = asset_path.components().last().expect("Asset file was empty?");
        let asset_filename = asset_filename_component.as_os_str().to_str().expect("Couldn't turn filename into string");

        if asset_filename.contains("~") { continue }

        let asset_constname = asset_filename
            .replace("-", "_")
            .replace(".", "_")
            .to_uppercase();

        let dest_path = Path::new(&out_dir).join(format!("{}.rs", asset_filename));
        let mut file = File::create(&dest_path).unwrap();

        let contents = fs::read_to_string(asset_path).expect("Couldn't read asset file")
            .replace("\\", "\\\\")
            .replace("\"", "\\\"");

        write!(
            &mut file,
            "static {}: &str = \"{}\";\n",
            asset_constname,
            contents
        ).expect("Failed to write asset rust constant");
    }
}
