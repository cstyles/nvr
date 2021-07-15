use neovim_lib::{Neovim, Session};
use std::env;
use std::process::{exit, Command};

fn main() {
    let arg = env::args().nth(1);

    match env::var("NVIM_LISTEN_ADDRESS") {
        Ok(listen_address) => connect_to_neovim_process(listen_address, arg),
        Err(_) => launch_new_neovim_process(arg),
    };
}

fn connect_to_neovim_process(listen_address: String, arg: Option<String>) {
    let _nvim = connect_to_nvim(&listen_address);
    println!("Connected!");
    println!("arg = {:?}", arg);
}

fn connect_to_nvim(address: &str) -> Neovim {
    let mut session = Session::new_unix_socket(address).unwrap();
    session.start_event_loop();
    Neovim::new(session)
}

fn launch_new_neovim_process(arg: Option<String>) {
    let return_code = Command::new("nvim")
        .args(arg)
        .spawn()
        .expect("failed to lanuch neovim")
        .wait()
        .unwrap()
        .code()
        .unwrap_or(1);

    exit(return_code);
}
