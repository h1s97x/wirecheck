// SPDX-License-Identifier: MIT
// Copyright (c) 2026 h1s97x

//! Built-in probe implementations for M1.
//!
//! M1 deliberately uses only the standard library: no async runtime, no TLS
//! stack and no raw-socket crate. HTTP is therefore plaintext only, and ICMP
//! starts at the system `ping` fallback (see `SPEC.md` §11).

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::process::Command;
use std::time::{Duration, Instant};
use wirecheck_core::observation::Observation;
use wirecheck_core::runner::{ProbeExecutor, RunContext};
use wirecheck_core::{ProbeMethod, ProbeSpec};

pub struct BuiltinProbes;

impl ProbeExecutor for BuiltinProbes {
    fn observe(&self, spec: &ProbeSpec, context: &RunContext) -> Observation {
        let started = Instant::now();
        let mut observation = match spec.kind.as_str() {
            "dns" => dns(&spec.with, context),
            "http" => http(&spec.with, context),
            "icmp" => icmp(&spec.with, context),
            "tcp" => tcp(&spec.with, context),
            other => Observation::failed(format!(
                "probe kind `{other}` is planned for a later milestone"
            )),
        };
        observation.elapsed_ms = started.elapsed().as_millis() as u64;
        observation
    }
}

fn string(with: &BTreeMap<String, toml::Value>, key: &str) -> Option<String> {
    match with.get(key) {
        Some(toml::Value::String(value)) => Some(value.clone()),
        Some(toml::Value::Integer(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn integer(with: &BTreeMap<String, toml::Value>, key: &str, default: u16) -> u16 {
    with.get(key)
        .and_then(|value| value.as_integer())
        .and_then(|value| u16::try_from(value).ok())
        .unwrap_or(default)
}

fn connect(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, String> {
    let address = (host, port)
        .to_socket_addrs()
        .map_err(|error| format!("cannot resolve `{host}`: {error}"))?
        .next()
        .ok_or_else(|| format!("no address for `{host}`"))?;
    TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| format!("cannot connect to {address}: {error}"))
}

fn tcp_reachable(host: &str, port: u16, timeout: Duration) -> bool {
    match (host, port).to_socket_addrs() {
        Ok(mut addresses) => match addresses.next() {
            Some(address) => TcpStream::connect_timeout(&address, timeout).is_ok(),
            None => false,
        },
        Err(_) => false,
    }
}

fn dns(with: &BTreeMap<String, toml::Value>, context: &RunContext) -> Observation {
    let query = string(with, "query").unwrap_or_else(|| "example.com".to_string());
    match string(with, "server") {
        Some(server) => query_resolver(&server, &query, context.timeout),
        None => system_resolve(&query),
    }
}

fn query_resolver(server: &str, query: &str, timeout: Duration) -> Observation {
    let address_text = if server.contains(':') {
        server.to_string()
    } else {
        format!("{server}:53")
    };
    let address = match address_text.to_socket_addrs() {
        Ok(mut addresses) => match addresses.next() {
            Some(address) => address,
            None => return Observation::failed(format!("no address for `{address_text}`")),
        },
        Err(error) => {
            return Observation::failed(format!("cannot resolve `{address_text}`: {error}"))
        }
    };
    let packet = match build_dns_query(query) {
        Ok(packet) => packet,
        Err(error) => return Observation::failed(error),
    };
    let bind_address = if address.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = match UdpSocket::bind(bind_address) {
        Ok(socket) => socket,
        Err(error) => return Observation::failed(format!("cannot open UDP socket: {error}")),
    };
    if let Err(error) = socket.set_read_timeout(Some(timeout)) {
        return Observation::failed(format!("cannot set read timeout: {error}"));
    }
    if let Err(error) = socket.set_write_timeout(Some(timeout)) {
        return Observation::failed(format!("cannot set write timeout: {error}"));
    }
    if let Err(error) = socket.send_to(&packet, address) {
        return Observation::failed(format!("DNS query failed: {error}"));
    }
    let mut buffer = [0u8; 2048];
    let (size, _) = match socket.recv_from(&mut buffer) {
        Ok(result) => result,
        Err(error) => return Observation::failed(format!("DNS response failed: {error}")),
    };
    let answers = match parse_answer_count(&buffer[..size]) {
        Ok(answers) => answers,
        Err(error) => return Observation::failed(error),
    };
    Observation::observed()
        .with("status", "ok")
        .with("answers", answers.to_string())
        .with("query", query)
        .with("server", address_text)
}

fn system_resolve(query: &str) -> Observation {
    match (query, 0u16).to_socket_addrs() {
        Ok(addresses) => Observation::observed()
            .with("status", "ok")
            .with("answers", addresses.count().to_string())
            .with("query", query)
            .with("server", "system"),
        Err(error) => Observation::failed(format!("system resolver failed: {error}")),
    }
}

fn build_dns_query(name: &str) -> Result<Vec<u8>, String> {
    let mut packet = Vec::new();
    packet.extend_from_slice(&0x1234u16.to_be_bytes()); // transaction id
    packet.extend_from_slice(&0x0100u16.to_be_bytes()); // recursion desired
    packet.extend_from_slice(&1u16.to_be_bytes()); // one question
    packet.extend_from_slice(&0u16.to_be_bytes()); // answers
    packet.extend_from_slice(&0u16.to_be_bytes()); // authority
    packet.extend_from_slice(&0u16.to_be_bytes()); // additional
    for label in name.trim_end_matches('.').split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(format!("invalid DNS name `{name}`"));
        }
        packet.push(label.len() as u8);
        packet.extend_from_slice(label.as_bytes());
    }
    packet.push(0);
    packet.extend_from_slice(&1u16.to_be_bytes()); // A
    packet.extend_from_slice(&1u16.to_be_bytes()); // IN
    Ok(packet)
}

fn skip_name(response: &[u8], position: &mut usize) -> Result<(), String> {
    loop {
        if *position >= response.len() {
            return Err("truncated DNS name".to_string());
        }
        let length = response[*position];
        if length & 0xC0 == 0xC0 {
            if *position + 1 >= response.len() {
                return Err("truncated DNS compression pointer".to_string());
            }
            *position += 2;
            return Ok(());
        }
        if length == 0 {
            *position += 1;
            return Ok(());
        }
        *position += 1 + usize::from(length);
        if *position > response.len() {
            return Err("truncated DNS label".to_string());
        }
    }
}

fn parse_answer_count(response: &[u8]) -> Result<usize, String> {
    if response.len() < 12 {
        return Err("short DNS response".to_string());
    }
    let questions = usize::from(u16::from_be_bytes([response[4], response[5]]));
    let answers = usize::from(u16::from_be_bytes([response[6], response[7]]));
    let mut position = 12;
    for _ in 0..questions {
        skip_name(response, &mut position)?;
        if position + 4 > response.len() {
            return Err("truncated DNS question".to_string());
        }
        position += 4;
    }
    for _ in 0..answers {
        skip_name(response, &mut position)?;
        if position + 10 > response.len() {
            return Err("truncated DNS answer".to_string());
        }
        let data_length = usize::from(u16::from_be_bytes([
            response[position + 8],
            response[position + 9],
        ]));
        position += 10 + data_length;
        if position > response.len() {
            return Err("truncated DNS rdata".to_string());
        }
    }
    Ok(answers)
}

fn http(with: &BTreeMap<String, toml::Value>, context: &RunContext) -> Observation {
    let url = match string(with, "url") {
        Some(url) => url,
        None => return Observation::failed("http probe needs `with.url`"),
    };
    if url.starts_with("https://") {
        return Observation::failed("https is not supported by the M1 std-only HTTP probe");
    }
    let rest = url.strip_prefix("http://").unwrap_or(&url);
    let (authority, path) = match rest.find('/') {
        Some(split) => (&rest[..split], &rest[split..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, raw_port)) => match raw_port.parse::<u16>() {
            Ok(port) => (host, port),
            Err(_) => (authority, 80),
        },
        None => (authority, 80),
    };
    let method = string(with, "method").unwrap_or_else(|| "GET".to_string());
    let mut stream = match connect(host, port, context.timeout) {
        Ok(stream) => stream,
        Err(error) => return Observation::failed(error),
    };
    if let Err(error) = stream.set_read_timeout(Some(context.timeout)) {
        return Observation::failed(format!("cannot set read timeout: {error}"));
    }
    if let Err(error) = stream.set_write_timeout(Some(context.timeout)) {
        return Observation::failed(format!("cannot set write timeout: {error}"));
    }
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: wirecheck/m1\r\nAccept: */*\r\n\r\n"
    );
    if let Err(error) = stream.write_all(request.as_bytes()) {
        return Observation::failed(format!("HTTP request failed: {error}"));
    }
    let mut raw = Vec::new();
    if let Err(error) = stream.read_to_end(&mut raw) {
        return Observation::failed(format!("HTTP response failed: {error}"));
    }
    let text = String::from_utf8_lossy(&raw);
    let status = http_status(&text);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, body)| body)
        .unwrap_or("")
        .chars()
        .take(65_536)
        .collect::<String>();
    Observation::observed()
        .with("status", status)
        .with("body", body)
        .with("url", url)
        .with("body_bytes", raw.len().to_string())
}

fn http_status(response: &str) -> String {
    response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("000")
        .to_string()
}

fn icmp(with: &BTreeMap<String, toml::Value>, context: &RunContext) -> Observation {
    let target = string(with, "target").unwrap_or_else(|| "127.0.0.1".to_string());
    let port = integer(with, "port", 80);
    match system_ping(&target, context.timeout) {
        Some(loss) => Observation::observed()
            .using(ProbeMethod::SystemFallback)
            .with("status", "ok")
            .with("packet_loss", loss.to_string())
            .with("target", target)
            .with("method", "system_ping"),
        None => {
            // Native raw sockets need platform-specific code, so M1 starts at
            // the system fallback and degrades to a TCP reachability estimate.
            // The chosen step is always recorded in `Finding.method`.
            let reachable = tcp_reachable(&target, port, context.timeout);
            Observation::observed()
                .using(ProbeMethod::TcpApproximation)
                .with("status", if reachable { "ok" } else { "unreachable" })
                .with("packet_loss", if reachable { "0.0" } else { "1.0" })
                .with("target", target)
                .with("port", port.to_string())
                .with("method", "tcp_approximation")
        }
    }
}

fn system_ping(target: &str, timeout: Duration) -> Option<f64> {
    let mut command = Command::new("ping");
    if cfg!(windows) {
        command
            .arg("-n")
            .arg("1")
            .arg("-w")
            .arg(timeout.as_millis().max(1000).to_string());
    } else {
        command
            .arg("-c")
            .arg("1")
            .arg("-W")
            .arg(timeout.as_secs().max(1).to_string());
    }
    let output = command.arg(target).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    if text.contains("0% packet loss") || text.contains("lost = 0") {
        Some(0.0)
    } else {
        Some(1.0)
    }
}

fn tcp(with: &BTreeMap<String, toml::Value>, context: &RunContext) -> Observation {
    let host = string(with, "host")
        .or_else(|| string(with, "target"))
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = integer(with, "port", 80);
    let reachable = tcp_reachable(&host, port, context.timeout);
    Observation::observed()
        .with("status", if reachable { "connected" } else { "refused" })
        .with("host", host)
        .with("port", port.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> RunContext {
        RunContext {
            timeout: Duration::from_millis(300),
            retries: 0,
            concurrency: 1,
        }
    }

    #[test]
    fn dns_query_has_a_valid_wire_header() {
        let packet = build_dns_query("example.com").unwrap();
        assert_eq!(&packet[..2], &[0x12, 0x34]);
        assert_eq!(&packet[2..4], &[0x01, 0x00]);
        assert_eq!(&packet[4..6], &[0x00, 0x01]);
        assert!(packet.ends_with(&[0x00, 0x01, 0x00, 0x01]));
        assert!(build_dns_query("bad..name").is_err());
    }

    #[test]
    fn dns_answers_with_compressed_names_are_parsed() {
        let mut response = vec![0u8; 12];
        response[2..4].copy_from_slice(&0x8180u16.to_be_bytes());
        response[4..6].copy_from_slice(&1u16.to_be_bytes());
        response[6..8].copy_from_slice(&1u16.to_be_bytes());
        response.push(7);
        response.extend_from_slice(b"example");
        response.push(3);
        response.extend_from_slice(b"com");
        response.push(0);
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&[0xC0, 0x0C]);
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&1u16.to_be_bytes());
        response.extend_from_slice(&60u32.to_be_bytes());
        response.extend_from_slice(&4u16.to_be_bytes());
        response.extend_from_slice(&[192, 0, 2, 1]);
        assert_eq!(parse_answer_count(&response).unwrap(), 1);
        assert!(parse_answer_count(&response[..11]).is_err());
    }

    #[test]
    fn http_status_line_is_parsed() {
        assert_eq!(http_status("HTTP/1.1 301 Moved\r\n\r\nbody"), "301");
        assert_eq!(http_status("garbage"), "000");
    }

    #[test]
    fn https_is_rejected_with_a_clear_error() {
        let mut with = BTreeMap::new();
        with.insert(
            "url".to_string(),
            toml::Value::String("https://example.com".to_string()),
        );
        let observation = http(&with, &context());
        assert!(observation.error.unwrap().contains("https"));
    }

    #[test]
    fn tcp_probe_detects_an_open_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let mut with = BTreeMap::new();
        with.insert(
            "host".to_string(),
            toml::Value::String("127.0.0.1".to_string()),
        );
        with.insert("port".to_string(), toml::Value::Integer(i64::from(port)));
        let observation = tcp(&with, &context());
        assert_eq!(observation.fields["status"], "connected");
    }
}
