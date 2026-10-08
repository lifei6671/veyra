#[cfg(target_os = "macos")]
fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = match args.as_slice() {
        [] => veyra_helper::production::serve_installed(),
        [action] if action == "serve" => veyra_helper::production::serve_installed(),
        [action] if action == "install" => veyra_helper::production::install(),
        [action] if action == "uninstall" => veyra_helper::production::uninstall(),
        _ => Err(veyra_helper::production::Error::InvalidRequest),
    };
    if let Err(error) = result {
        eprintln!("{error:?}"); // 仅固定错误码，不输出配置/凭据。
        std::process::exit(1);
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("Unavailable: macOS helper only");
    std::process::exit(1);
}
