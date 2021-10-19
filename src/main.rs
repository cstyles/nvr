use neovim_lib::{CallError, Neovim, NeovimApi, Session, Value};
use std::env;
use std::ffi::OsString;
use std::process::{exit, Command};
use std::sync::mpsc::Receiver;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match env::var_os("NVIM_LISTEN_ADDRESS") {
        Some(listen_address) => open_in_existing_neovim(listen_address, args).unwrap(),
        None => launch_new_neovim_process(args),
    };
}

fn open_in_existing_neovim(listen_address: OsString, args: Vec<String>) -> Result<(), CallError> {
    let (mut nvim, receiver) = connect_to_nvim(listen_address);

    if args.is_empty() {
        return nvim.command("enew");
    }

    let mut commands = vec![];
    let cd = std::env::var("PWD").expect("no PWD");

    for arg in args.iter() {
        if arg.starts_with('+') {
            let command = arg.strip_prefix('+').expect("always Some");
            commands.push(command);
        } else {
            let command = format!(
                "split | lcd {} | edit {} | setlocal bufhidden=delete",
                cd, arg
            );
            nvim.command(&command)?;
        }
    }

    for command in commands {
        nvim.command(command)?;
    }

    // TODO: this will only wait for one buffer to close
    wait_for_buffer_to_close(&mut nvim, receiver)?;

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
    let message = nvim.session.call("nvim_get_api_info", vec![]).ok()?;
    let array = message.as_array()?;
    let first = array.first()?;
    first.as_u64()
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

fn connect_to_nvim(address: OsString) -> (Neovim, Receiver<(String, Vec<Value>)>) {
    let mut session = Session::new_unix_socket(address).unwrap();

    // Store a Receiver that we can use to read responses back from neovim
    let receiver = session.start_event_loop_channel();

    (Neovim::new(session), receiver)
}

fn launch_new_neovim_process(args: Vec<String>) {
    let return_code = Command::new("nvim")
        .args(args)
        .spawn()
        .expect("failed to launch neovim")
        .wait()
        .unwrap()
        .code()
        .unwrap_or(1);

    exit(return_code);
}
