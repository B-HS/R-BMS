#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    rbms_player::run(std::env::args())
}
