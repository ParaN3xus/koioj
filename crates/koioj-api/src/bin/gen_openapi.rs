use koioj_api::route::ApiDoc;
use utoipa::OpenApi;

#[allow(unreachable_code)]
fn main() {
    #[cfg(feature = "embed-frontend")]
    panic!("generating openapi docs with `embed-frontend` feature on!");

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    std::fs::write(path, ApiDoc::openapi().to_pretty_json().unwrap())
        .expect("Failed to write openapi.json");
}
