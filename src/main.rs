use neovim_lib::{CallError, Neovim, NeovimApi, Session, Value};
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::process::{exit, Command};
use std::sync::mpsc::Receiver;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    match env::var_os("NVIM") {
        Some(listen_address) => open_in_existing_neovim(listen_address, args).unwrap(),
        None => launch_new_neovim_process(args),
    };
}

fn open_in_existing_neovim(listen_address: OsString, args: Vec<String>) -> Result<(), CallError> {
    let (mut nvim, receiver) = connect_to_nvim(listen_address);

    let channel_id = get_channel_id(&mut nvim).unwrap_or_else(|| {
        eprintln!("Couldn't acquire channel ID");
        exit(2)
    });

    if args.is_empty() {
        nvim.command("split | enew | setlocal bufhidden=delete")?;
        set_up_augroup(&mut nvim, channel_id)?;

        let buffer_number = nvim.get_current_buf()?.get_number(&mut nvim)?;
        let buffer_numbers = HashSet::from([buffer_number]);

        return wait_for_buffers_to_close(receiver, buffer_numbers);
    }

    let mut commands = vec![];
    let mut buffer_numbers = HashSet::with_capacity(args.len());
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
            set_up_augroup(&mut nvim, channel_id)?;

            let buffer_number = nvim.get_current_buf()?.get_number(&mut nvim)?;
            buffer_numbers.insert(buffer_number);
        }
    }

    for command in commands {
        nvim.command(command)?;
    }

    wait_for_buffers_to_close(receiver, buffer_numbers)?;

    Ok(())
}

/// Waits for a response from neovim, triggered by closing the buffer
fn wait_for_buffers_to_close(
    receiver: Receiver<(String, Vec<Value>)>,
    mut buffer_numbers: HashSet<i64>,
) -> Result<(), CallError> {
    while !buffer_numbers.is_empty() {
        let message = receiver.recv().unwrap();

        match message.1.as_slice() {
            [Value::Integer(num)] => {
                let num = num.as_i64().expect("Buffer number wasn't an integer.");
                buffer_numbers.remove(&num);
            }
            anything_else => eprintln!("Received unexpected message: {:?}", anything_else),
        };
    }

    Ok(())
}

fn get_channel_id(nvim: &mut Neovim) -> Option<u64> {
    let message = nvim.session.call("nvim_get_api_info", vec![]).ok()?;
    let array = message.as_array()?;
    let first = array.first()?;
    first.as_u64()
}

/// Sets up an autocmd group that will listen for BufDelete events for the current buffer
/// and send a message back to us when the buffer is closed.
fn set_up_augroup(nvim: &mut Neovim, channel_id: u64) -> Result<(), CallError> {
    let command = [
        "augroup nvr".into(),
        format!(
            "autocmd BufDelete <buffer> silent! call rpcnotify({}, \"BufDelete\", bufnr())",
            channel_id
        ),
        "augroup END".into(),
    ]
    .join(" | ");

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
