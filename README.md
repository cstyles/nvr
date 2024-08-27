# nvr

If invoked outside of Neovim, `nvr [options]` simply invokes `nvim [options]`. If invoked from within Neovim (such as in a terminal buffer), `nvr [options]` will open a new, horizontally split window in the existing Neovim session.

## Usage

`nvr` is intended to be used as your `$EDITOR` or `$GIT_EDITOR`. When you invoke commands like `git commit`, `git rebase --interactive`, etc. within a Neovim terminal buffer, a new split window appears in the existing Neovim session (instead of in a nested Neovim session). After writing a commit message, save and exit the window with, e.g., `:x`. That will delete the buffer and signal to `git commit` that you're finished authoring your commit message.

`nvr` can be invoked with multiple files. You must save and exit all buffers opened by `nvr` before the process will exit and return control to your shell.

`nvr` supports the `+N` / `+command` options as they work in Vim/Neovim.

`nvr` supports the `-q` option as it works in Vim/Neovim and thus is usable via commands like [`git jump grep`](https://git.kernel.org/pub/scm/git/git.git/tree/contrib/git-jump/README).
