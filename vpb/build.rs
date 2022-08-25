fn main() {
    prost_build::Config::new()
        .out_dir("pb/")
        .compile_protos(&["pb/pb.proto"], &["pb"])
        .unwrap();
}
