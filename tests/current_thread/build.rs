fn main() {
    tonic_build::configure()
        .local_executor(true)
        .compile(&["proto/test.proto"], &["proto"])
        .unwrap();
}
