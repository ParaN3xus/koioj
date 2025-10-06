include!(concat!(env!("OUT_DIR"), "/static_files.rs"));

pub fn get_file(path: &str) -> Option<&[u8]> {
    DIST_FILES.get(path).copied()
}
