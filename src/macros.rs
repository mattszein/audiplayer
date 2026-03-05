#[macro_export]
macro_rules! project_file {
    ($path:expr) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
    };
}
