fn main() {
    tonic_build::configure()
        .single_threaded(true)
        .compile(&["proto/test.proto"], &["proto"])
        .unwrap();
}
