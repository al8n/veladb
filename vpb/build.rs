fn main() {
    prost_build::Config::new()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[serde(rename_all = \"kebab-case\")]")
        .out_dir("src/")
        .compile_protos(&["pb/pb.proto"], &["pb"])
        .unwrap();
}
