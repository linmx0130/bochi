use std::io;
use std::process::Command;

pub fn get_adb_command(serial: Option<&str>) -> io::Result<Command> {
    let mut cmd = Command::new("adb");
    if let Some(s) = serial {
        cmd.arg("-s").arg(s);
    }
    Ok(cmd)
}

pub fn format_adb_error(e: &io::Error) -> String {
    if e.kind() == io::ErrorKind::NotFound {
        "adb is not available in the $PATH directories".to_string()
    } else {
        format!("Failed to execute adb: {}", e)
    }
}
