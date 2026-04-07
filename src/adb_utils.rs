use std::io;
use std::process::Command;

/// Parse a remote string in format "host:port"
/// Returns (host, port) - port must be explicitly specified
pub fn parse_remote(remote: &str) -> Result<(&str, &str), String> {
    if remote.is_empty() {
        return Err("Remote address cannot be empty".to_string());
    }
    
    // Handle IPv6 addresses in brackets like [::1]:5037
    if remote.starts_with('[') {
        if let Some(idx) = remote.find("]:") {
            let host = &remote[..idx + 1]; // Include the brackets
            let port = &remote[idx + 2..];
            if port.is_empty() {
                return Err("Port cannot be empty".to_string());
            }
            return Ok((host, port));
        } else if remote.ends_with(']') {
            // IPv6 without port - error
            return Err("Port must be specified (e.g., [::1]:5037)".to_string());
        }
    }
    
    // Standard host:port format
    if let Some(idx) = remote.rfind(':') {
        // Check if this is an IPv6 address without brackets (contains multiple colons)
        let colon_count = remote.matches(':').count();
        if colon_count == 1 {
            // Regular host:port
            let host = &remote[..idx];
            let port = &remote[idx + 1..];
            if host.is_empty() {
                return Err("Host cannot be empty".to_string());
            }
            if port.is_empty() {
                return Err("Port cannot be empty".to_string());
            }
            return Ok((host, port));
        } else {
            // IPv6 without brackets - error
            return Err("IPv6 addresses must be enclosed in brackets (e.g., [::1]:5037)".to_string());
        }
    }
    
    // Just host without port - error
    Err("Port must be specified in host:port format (e.g., 127.0.0.1:5037)".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_valid_ipv4() {
        assert_eq!(parse_remote("127.0.0.1:5037"), Ok(("127.0.0.1", "5037")));
        assert_eq!(parse_remote("192.168.1.100:5555"), Ok(("192.168.1.100", "5555")));
    }

    #[test]
    fn test_parse_remote_valid_hostname() {
        assert_eq!(parse_remote("localhost:5037"), Ok(("localhost", "5037")));
        assert_eq!(parse_remote("adb-server.example.com:5555"), Ok(("adb-server.example.com", "5555")));
    }

    #[test]
    fn test_parse_remote_valid_ipv6() {
        assert_eq!(parse_remote("[::1]:5037"), Ok(("[::1]", "5037")));
        assert_eq!(parse_remote("[2001:db8::1]:5555"), Ok(("[2001:db8::1]", "5555")));
    }

    #[test]
    fn test_parse_remote_missing_port() {
        assert!(parse_remote("127.0.0.1").is_err());
        assert!(parse_remote("localhost").is_err());
        assert!(parse_remote("[::1]").is_err());
    }

    #[test]
    fn test_parse_remote_empty() {
        assert!(parse_remote("").is_err());
    }

    #[test]
    fn test_parse_remote_empty_host() {
        assert!(parse_remote(":5037").is_err());
    }

    #[test]
    fn test_parse_remote_empty_port() {
        assert!(parse_remote("127.0.0.1:").is_err());
        assert!(parse_remote("localhost:").is_err());
        assert!(parse_remote("[::1]:").is_err());
    }

    #[test]
    fn test_parse_remote_ipv6_without_brackets() {
        assert!(parse_remote("::1:5037").is_err());
        assert!(parse_remote("2001:db8::1:5037").is_err());
    }

    #[test]
    fn test_parse_remote_multiple_colons_not_ipv6() {
        // This should fail because it looks like IPv6 without brackets
        assert!(parse_remote("a:b:c:5037").is_err());
    }
}

pub fn get_adb_command(serial: Option<&str>, remote: Option<&str>) -> io::Result<Command> {
    let mut cmd = Command::new("adb");
    
    // Add remote host/port if specified (must come before -s)
    if let Some(r) = remote {
        match parse_remote(r) {
            Ok((host, port)) => {
                cmd.arg("-H").arg(host);
                cmd.arg("-P").arg(port);
            }
            Err(e) => {
                // Return an io::Error with custom message
                return Err(io::Error::new(io::ErrorKind::InvalidInput, e));
            }
        }
    }
    
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
