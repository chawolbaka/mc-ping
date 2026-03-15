use std::io::{self, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

const MINECRAFT_DEFAULT_PORT: u16 = 25565;

// Almost entirely written by ChatGPT

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IpFamily {
    Any,
    V4,
    V6,
}

impl IpFamily {
    fn matches(self, addr: &SocketAddr) -> bool {
        match (self, addr) {
            (IpFamily::Any, _) => true,
            (IpFamily::V4, SocketAddr::V4(_)) => true,
            (IpFamily::V6, SocketAddr::V6(_)) => true,
            _ => false,
        }
    }
}

pub fn resolve_host_with_family(host: &str, family: IpFamily) -> io::Result<SocketAddr> {
    let addrs = to_socket_addrs_with_family(host, family)?;
    addrs.into_iter().next().ok_or_else(|| {
        io::Error::new(
            ErrorKind::NotFound,
            format!(
                "Ping request could not find host {}. Please check the name and try again.",
                strip_port(host)
            ),
        )
    })
}

pub fn to_socket_addrs_with_family(host: &str, family: IpFamily) -> io::Result<Vec<SocketAddr>> {
    let host = with_default_port(host);
    if let Ok(addr) = host.parse::<SocketAddr>() {
        if family.matches(&addr) {
            return Ok(vec![addr]);
        }
        return Ok(Vec::new());
    }

    let (node, port) = split_host_port(&host)?;
    lookup_host_with_family(&node, port, family)
}

pub fn strip_port(s: &str) -> &str {
    if let Some(stripped) = strip_bracketed_port(s) {
        return stripped;
    }

    let colon_count = s.matches(':').count();
    if colon_count <= 1 {
        if let Some((host, port)) = s.rsplit_once(':') {
            if port.chars().all(|c| c.is_ascii_digit()) {
                return host;
            }
        }
    }

    s
}

fn strip_bracketed_port(s: &str) -> Option<&str> {
    if !s.starts_with('[') {
        return None;
    }
    let end = s.find(']')?;
    let rest = &s[end + 1..];
    if rest.starts_with(':') && rest[1..].chars().all(|c| c.is_ascii_digit()) {
        return Some(&s[1..end]);
    }
    None
}

fn split_host_port(s: &str) -> io::Result<(String, u16)> {
    if s.starts_with('[') {
        let end = s.find(']').ok_or_else(|| invalid_input("invalid socket address"))?;
        let host = &s[1..end];
        let rest = &s[end + 1..];
        if !rest.starts_with(':') {
            return Err(invalid_input("invalid socket address"));
        }
        let port = rest[1..]
            .parse::<u16>()
            .map_err(|_| invalid_input("invalid port"))?;
        return Ok((host.to_string(), port));
    }

    let (host, port_str) = s
        .rsplit_once(':')
        .ok_or_else(|| invalid_input("invalid socket address"))?;
    if host.is_empty() {
        return Err(invalid_input("invalid socket address"));
    }
    let port = port_str
        .parse::<u16>()
        .map_err(|_| invalid_input("invalid port"))?;
    Ok((host.to_string(), port))
}

fn invalid_input(message: &'static str) -> io::Error {
    io::Error::new(ErrorKind::InvalidInput, message)
}

fn with_default_port(host: &str) -> String {
    if host.is_empty() {
        return host.to_string();
    }

    if host.starts_with('[') {
        if let Some(end) = host.find(']') {
            let rest = &host[end + 1..];
            if rest.starts_with(':') && rest[1..].chars().all(|c| c.is_ascii_digit()) {
                return host.to_string();
            }
            if rest.is_empty() {
                return format!("{host}:{MINECRAFT_DEFAULT_PORT}");
            }
        }
        return host.to_string();
    }

    let colon_count = host.matches(':').count();
    match colon_count {
        0 => format!("{host}:{MINECRAFT_DEFAULT_PORT}"),
        1 => host.to_string(),
        _ => format!("[{host}]:{MINECRAFT_DEFAULT_PORT}"),
    }
}

#[cfg(unix)]
fn lookup_host_with_family(host: &str, port: u16, family: IpFamily) -> io::Result<Vec<SocketAddr>> {
    use libc::{
        addrinfo, freeaddrinfo, getaddrinfo, sockaddr_in, sockaddr_in6, AF_INET, AF_INET6,
        AF_UNSPEC, AI_ADDRCONFIG, AI_NUMERICSERV, SOCK_STREAM,
    };
    use std::ffi::CString;
    use std::mem;
    use std::ptr;

    let host_c = CString::new(host).map_err(|_| invalid_input("invalid host"))?;
    let service_c = CString::new(port.to_string()).map_err(|_| invalid_input("invalid port"))?;

    let mut hints: addrinfo = unsafe { mem::zeroed() };
    hints.ai_family = match family {
        IpFamily::Any => AF_UNSPEC,
        IpFamily::V4 => AF_INET,
        IpFamily::V6 => AF_INET6,
    };
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_NUMERICSERV;
    if family == IpFamily::Any {
        hints.ai_flags |= AI_ADDRCONFIG;
    }

    let mut res: *mut addrinfo = ptr::null_mut();
    let err = unsafe { getaddrinfo(host_c.as_ptr(), service_c.as_ptr(), &hints, &mut res) };
    if err != 0 {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("DNS lookup failed: {err}"),
        ));
    }

    struct AddrInfoGuard(*mut addrinfo);
    impl Drop for AddrInfoGuard {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    freeaddrinfo(self.0);
                }
            }
        }
    }
    let _guard = AddrInfoGuard(res);

    let mut addrs = Vec::new();
    let mut cur = res;
    while !cur.is_null() {
        let ai = unsafe { &*cur };
        if !ai.ai_addr.is_null() {
            if ai.ai_family == AF_INET {
                let sockaddr = unsafe { *(ai.ai_addr as *const sockaddr_in) };
                let ip = Ipv4Addr::from(u32::from_be(sockaddr.sin_addr.s_addr));
                let port = u16::from_be(sockaddr.sin_port);
                addrs.push(SocketAddr::V4(SocketAddrV4::new(ip, port)));
            } else if ai.ai_family == AF_INET6 {
                let sockaddr = unsafe { *(ai.ai_addr as *const sockaddr_in6) };
                let ip = Ipv6Addr::from(sockaddr.sin6_addr.s6_addr);
                let port = u16::from_be(sockaddr.sin6_port);
                let flowinfo = u32::from_be(sockaddr.sin6_flowinfo);
                let scope_id = sockaddr.sin6_scope_id;
                addrs.push(SocketAddr::V6(SocketAddrV6::new(
                    ip, port, flowinfo, scope_id,
                )));
            }
        }
        cur = ai.ai_next;
    }

    Ok(addrs)
}

#[cfg(windows)]
fn lookup_host_with_family(host: &str, port: u16, family: IpFamily) -> io::Result<Vec<SocketAddr>> {
    use std::mem;
    use std::ptr;
    use std::sync::OnceLock;
    use windows_sys::Win32::Networking::WinSock::{
        ADDRINFOW, FreeAddrInfoW, GetAddrInfoW, SOCKADDR_IN, SOCKADDR_IN6, WSAStartup, WSADATA,
        AF_INET, AF_INET6, AF_UNSPEC, AI_ADDRCONFIG, AI_NUMERICSERV, SOCK_STREAM,
    };

    if host.contains('\0') {
        return Err(invalid_input("invalid host"));
    }

    static START: OnceLock<i32> = OnceLock::new();
    let code = *START.get_or_init(|| unsafe {
        let mut data = mem::MaybeUninit::<WSADATA>::zeroed();
        WSAStartup(0x202, data.as_mut_ptr())
    });
    if code != 0 {
        return Err(io::Error::from_raw_os_error(code));
    }

    let mut host_w: Vec<u16> = host.encode_utf16().collect();
    host_w.push(0);
    let mut service_w: Vec<u16> = port.to_string().encode_utf16().collect();
    service_w.push(0);

    let mut hints: ADDRINFOW = unsafe { mem::zeroed() };
    hints.ai_family = match family {
        IpFamily::Any => AF_UNSPEC as i32,
        IpFamily::V4 => AF_INET as i32,
        IpFamily::V6 => AF_INET6 as i32,
    };
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_flags = AI_NUMERICSERV as i32;
    if family == IpFamily::Any {
        hints.ai_flags |= AI_ADDRCONFIG as i32;
    }

    let mut res: *mut ADDRINFOW = ptr::null_mut();
    let err = unsafe { GetAddrInfoW(host_w.as_ptr(), service_w.as_ptr(), &hints, &mut res) };
    if err != 0 {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("DNS lookup failed: {err}"),
        ));
    }

    struct AddrInfoGuard(*mut ADDRINFOW);
    impl Drop for AddrInfoGuard {
        fn drop(&mut self) {
            unsafe {
                if !self.0.is_null() {
                    FreeAddrInfoW(self.0);
                }
            }
        }
    }
    let _guard = AddrInfoGuard(res);

    let mut addrs = Vec::new();
    let mut cur = res;
    while !cur.is_null() {
        let ai = unsafe { &*cur };
        if !ai.ai_addr.is_null() {
            if ai.ai_family == AF_INET as i32 {
                let sockaddr = unsafe { *(ai.ai_addr as *const SOCKADDR_IN) };
                let ip = Ipv4Addr::from(u32::from_be(unsafe { sockaddr.sin_addr.S_un.S_addr }));
                let port = u16::from_be(sockaddr.sin_port);
                addrs.push(SocketAddr::V4(SocketAddrV4::new(ip, port)));
            } else if ai.ai_family == AF_INET6 as i32 {
                let sockaddr = unsafe { *(ai.ai_addr as *const SOCKADDR_IN6) };
                let ip = Ipv6Addr::from(unsafe { sockaddr.sin6_addr.u.Byte });
                let port = u16::from_be(sockaddr.sin6_port);
                let flowinfo = u32::from_be(sockaddr.sin6_flowinfo);
                let scope_id = unsafe { sockaddr.Anonymous.sin6_scope_id };
                addrs.push(SocketAddr::V6(SocketAddrV6::new(
                    ip, port, flowinfo, scope_id,
                )));
            }
        }
        cur = ai.ai_next;
    }

    Ok(addrs)
}

#[cfg(not(any(windows, unix)))]
fn lookup_host_with_family(host: &str, port: u16, family: IpFamily) -> io::Result<Vec<SocketAddr>> {
    use std::net::ToSocketAddrs;

    let iter = (host, port).to_socket_addrs()?;
    Ok(iter.filter(|addr| family.matches(addr)).collect())
}
