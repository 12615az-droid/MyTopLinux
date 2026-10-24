fn main() {
    if let Err(error) = mytop_linux::presentation::desktop::run() {
        eprintln!("MyTop: {error}");
        std::process::exit(1);
    }
}
