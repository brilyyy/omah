pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

pub fn parse_version(s: &str) -> Option<Version> {
    let s = s.strip_prefix('v').unwrap_or(s);
    let mut parts = s.splitn(3, '.');
    Some(Version {
        major: parts.next()?.parse().ok()?,
        minor: parts.next()?.parse().ok()?,
        patch: parts.next()?.parse().ok()?,
    })
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    let latest = parse_version(latest);
    let current = parse_version(current);
    match (latest, current) {
        (Some(l), Some(c)) => {
            l.major > c.major
                || (l.major == c.major && l.minor > c.minor)
                || (l.major == c.major && l.minor == c.minor && l.patch > c.patch)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        let v = parse_version("v0.3.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 3);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_parse_version_no_v() {
        let v = parse_version("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("abc").is_none());
        assert!(parse_version("v1.x.3").is_none());
        assert!(parse_version("").is_none());
    }

    #[test]
    fn test_is_newer() {
        assert!(is_newer("v0.4.0", "v0.3.0"));
        assert!(is_newer("v1.0.0", "v0.9.9"));
        assert!(is_newer("v0.3.1", "v0.3.0"));
        assert!(!is_newer("v0.3.0", "v0.3.0"));
        assert!(!is_newer("v0.2.0", "v0.3.0"));
        assert!(!is_newer("v0.3.0", "v0.4.0"));
    }

    #[test]
    fn test_is_newer_invalid_returns_false() {
        assert!(!is_newer("invalid", "v0.3.0"));
        assert!(!is_newer("v0.4.0", "invalid"));
    }

    #[test]
    fn test_current_version_parses() {
        assert!(parse_version(CURRENT_VERSION).is_some());
    }
}
