use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListeningSocket {
    pub protocol: String,
    pub address: String,
    pub port: u16,
    pub inode: String,
}

const TCP_LISTEN_STATE: &str = "0A";

pub fn list_listeners() -> Result<Vec<ListeningSocket>, String> {
    let mut sockets = Vec::new();
    sockets.extend(read_tcp_file("/proc/net/tcp", "tcp")?);
    sockets.extend(read_tcp_file("/proc/net/tcp6", "tcp6")?);
    Ok(sockets)
}

fn read_tcp_file(path: &str, protocol: &str) -> Result<Vec<ListeningSocket>, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("failed to read {path}: {error}")),
    };

    Ok(parse_tcp_table(&content, protocol))
}

fn parse_tcp_table(content: &str, protocol: &str) -> Vec<ListeningSocket> {
    content
        .lines()
        .skip(1)
        .filter_map(|line| parse_tcp_line(line, protocol))
        .collect()
}

fn parse_tcp_line(line: &str, protocol: &str) -> Option<ListeningSocket> {
    let columns = line.split_whitespace().collect::<Vec<_>>();
    let local_address = columns.get(1)?;
    let state = columns.get(3)?;
    let inode = columns.get(9)?;

    if *state != TCP_LISTEN_STATE {
        return None;
    }

    let (address_hex, port_hex) = local_address.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let address = match protocol {
        "tcp" => parse_ipv4(address_hex)?,
        "tcp6" => parse_ipv6(address_hex)?,
        _ => return None,
    };

    Some(ListeningSocket {
        protocol: protocol.to_string(),
        address,
        port,
        inode: inode.to_string(),
    })
}

fn parse_ipv4(value: &str) -> Option<String> {
    if value.len() != 8 {
        return None;
    }

    let raw = u32::from_str_radix(value, 16).ok()?;
    let bytes = raw.to_le_bytes();
    Some(Ipv4Addr::from(bytes).to_string())
}

fn parse_ipv6(value: &str) -> Option<String> {
    if value.len() != 32 {
        return None;
    }

    let mut bytes = [0_u8; 16];
    for index in 0..4 {
        let start = index * 8;
        let chunk = u32::from_str_radix(&value[start..start + 8], 16).ok()?;
        bytes[index * 4..index * 4 + 4].copy_from_slice(&chunk.to_le_bytes());
    }

    Some(Ipv6Addr::from(bytes).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_listening_ipv4_socket() {
        let table = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 0100007F:1435 00000000:0000 0A 00000000:00000000 00:00000000 00000000 1000 0 12345 1 0000000000000000 100 0 0 10 0\n";
        let sockets = parse_tcp_table(table, "tcp");

        assert_eq!(
            sockets,
            vec![ListeningSocket {
                protocol: "tcp".to_string(),
                address: "127.0.0.1".to_string(),
                port: 5173,
                inode: "12345".to_string(),
            }]
        );
    }

    #[test]
    fn skips_non_listening_socket() {
        let table = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n   0: 0100007F:1435 00000000:0000 01 00000000:00000000 00:00000000 00000000 1000 0 12345 1 0000000000000000 100 0 0 10 0\n";
        assert!(parse_tcp_table(table, "tcp").is_empty());
    }

    #[test]
    fn parses_any_ipv4_address() {
        assert_eq!(parse_ipv4("00000000"), Some("0.0.0.0".to_string()));
    }
}
