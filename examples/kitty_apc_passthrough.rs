//! Verify that a Windows ConPTY preserves a Kitty APC sequence emitted by a child.
//!
//! The executable must be run next to the bundled `conpty/` directory when the
//! bundled runtime is being tested. `scripts/windows_kitty_apc_probe.ps1`
//! prepares that layout automatically.

#[cfg(windows)]
fn main() {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::Read;
    use std::sync::mpsc;
    use std::time::Duration;

    let sequence = b"\x1b_Gi=42,a=q,t=d,f=24,s=1,v=1;AAAA\x1b\\";
    let script = r#"$s=[char]27+'_Gi=42,a=q,t=d,f=24,s=1,v=1;AAAA'+[char]27+'\'; $b=[Text.Encoding]::ASCII.GetBytes($s); [Console]::OpenStandardOutput().Write($b,0,$b.Length)"#;

    let pair = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("open ConPTY");
    let mut command = CommandBuilder::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        script,
    ]);
    let mut child = pair.slave.spawn_command(command).expect("spawn PowerShell");
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().expect("clone ConPTY reader");
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    let _ = sender.send(Ok(output));
                    return;
                }
                Ok(count) => {
                    output.extend_from_slice(&buffer[..count]);
                    if output
                        .windows(sequence.len())
                        .any(|window| window == sequence)
                    {
                        let _ = sender.send(Ok(output));
                        return;
                    }
                    if output.len() > 1024 * 1024 {
                        let _ = sender.send(Err("ConPTY output exceeded 1 MiB"));
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err("failed to read ConPTY output"));
                    eprintln!("read error: {error}");
                    return;
                }
            }
        }
    });

    let output = receiver.recv_timeout(Duration::from_secs(10));
    if output.is_err() {
        let _ = child.kill();
        panic!("Kitty APC did not pass through ConPTY within 10 seconds");
    }
    output.expect("probe result").expect("read ConPTY output");
    let _ = child.kill();
    println!("Kitty APC passthrough: OK");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("This probe only runs on Windows");
}
