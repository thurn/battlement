fn main() {
  if let Err(error) = battlement_ditto::run() {
    eprintln!("error: {error:#}");
    std::process::exit(1);
  }
}
