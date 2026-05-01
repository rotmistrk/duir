use std::io::Write;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// Copy text to the system clipboard via OSC 52 terminal escape sequence.
/// Works in iTerm, kitty, alacritty, tmux (set-clipboard on), and most modern terminals.
/// Also works over SSH.
pub fn copy_to_clipboard(text: &str) {
    let encoded = BASE64.encode(text.as_bytes());
    // OSC 52: \x1b]52;c;BASE64\x07
    let seq = format!("\x1b]52;c;{encoded}\x07");
    let mut stdout = std::io::stdout();
    stdout.write_all(seq.as_bytes()).ok();
    stdout.flush().ok();
}
