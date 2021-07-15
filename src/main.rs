use neovim_lib::{CallError, Neovim, NeovimApi, Session, Value};
use std::env;
use std::path::Path;
use std::process::{exit, Command};
use std::sync::mpsc::Receiver;

fn main() {
    let arg = env::args().nth(1);

    match env::var_os("NVIM_LISTEN_ADDRESS") {
        Some(listen_address) => open_in_existing_neovim(listen_address, arg).unwrap(),
        None => launch_new_neovim_process(arg),
    };
}

fn open_in_existing_neovim<T: AsRef<Path>>(
    listen_address: T,
    arg: Option<String>,
) -> Result<(), CallError> {
    let listen_address = listen_address.as_ref();
    let (mut nvim, receiver) = connect_to_nvim(listen_address);
    nvim.command("split")?;

    let result = match arg {
        Some(arg) => {
            let command = format!("edit {}", arg);
            nvim.command(&command)
                .and_then(|()| nvim.command("set bufhidden=delete"))
                .and_then(|()| wait_for_buffer_to_close(&mut nvim, receiver))
        }
        None => nvim.command("enew"),
    };

    if let Err(err) = result {
        eprintln!("{}", err);
    };

    Ok(())
}

fn wait_for_buffer_to_close(
    nvim: &mut Neovim,
    receiver: Receiver<(String, Vec<Value>)>,
) -> Result<(), CallError> {
    let channel_id = match get_channel_id(nvim) {
        Some(channel_id) => channel_id,
        None => {
            eprintln!("Couldn't acquire channel ID");
            exit(2)
        }
    };

    set_up_augroup(nvim, channel_id)?;

    // Wait for a response from neovim, triggered by closing the buffer
    // TODO: check that the response is actually what we expect?
    let _ = receiver.recv().unwrap();

    Ok(())
}

fn get_channel_id(nvim: &mut Neovim) -> Option<u64> {
    nvim.session
        .call("nvim_get_api_info", vec![])
        .as_ref()
        .map(Value::as_array)
        .ok()
        .flatten()
        .as_deref() // .map(Vec::as_slice)
        .map(|slice| slice.get(0))
        .flatten()
        .map(Value::as_u64)
        .flatten()
}

/// Sets up an autocmd group that will listen for BufDelete events and send a message back to us
/// when the buffer that we opened is closed.
fn set_up_augroup(nvim: &mut Neovim, channel_id: u64) -> Result<(), CallError> {
    let command = [
        "augroup nvr".into(),
        format!(
            "autocmd BufDelete <buffer> silent! call rpcnotify({}, \"BufDelete\")",
            channel_id
        ),
        "augroup END".into(),
    ]
    .join("|");

    nvim.command(&command)
}

fn connect_to_nvim<T: AsRef<Path>>(address: T) -> (Neovim, Receiver<(String, Vec<Value>)>) {
    let address = address.as_ref();
    let mut session = Session::new_unix_socket(address).unwrap();

    // Store a Receiver that we can use to read responses back from neovim
    let receiver = session.start_event_loop_channel();

    (Neovim::new(session), receiver)
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
