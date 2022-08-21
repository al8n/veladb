fn main() {
    prost_build::Config::new()
        .out_dir("src/")
        .compile_protos(&["pb/pb.proto"], &["pb"])
        .unwrap();
}
