fn main() {
    if let Err(e) = astronomicon_app::run() {
        eprintln!("erro fatal: {}", e);
        std::process::exit(1);
    }
}