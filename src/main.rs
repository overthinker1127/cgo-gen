fn main() {
    if let Err(error) = cgo_gen::cli::run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
