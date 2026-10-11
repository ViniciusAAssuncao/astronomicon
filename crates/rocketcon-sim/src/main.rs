fn main() {
    if let Err(err) = rocketcon_sim::run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}