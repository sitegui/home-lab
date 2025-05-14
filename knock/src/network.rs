use anyhow::{Context, ensure};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

/// Represent a network range of IPs.
///
/// For example, "192.168.1.0/24" represents the range from 192.168.1.0 to 192.168.1.255 (inclusive)
#[derive(Debug, Copy, Clone)]
pub struct Network(NetworkInner);

impl Network {
    pub fn includes(&self, ip: IpAddr) -> bool {
        match (self.0, ip) {
            (
                NetworkInner::V4 {
                    inclusive_start,
                    inclusive_end,
                },
                IpAddr::V4(ip),
            ) => inclusive_start <= ip && ip <= inclusive_end,
            (
                NetworkInner::V6 {
                    inclusive_start,
                    inclusive_end,
                },
                IpAddr::V6(ip),
            ) => inclusive_start <= ip && ip <= inclusive_end,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone)]
enum NetworkInner {
    V4 {
        inclusive_start: Ipv4Addr,
        inclusive_end: Ipv4Addr,
    },
    V6 {
        inclusive_start: Ipv6Addr,
        inclusive_end: Ipv6Addr,
    },
}

impl FromStr for Network {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (base, prefix_length) = match s.split_once('/') {
            None => (s, None),
            Some((base, prefix_length)) => {
                let prefix_length: u8 = prefix_length.parse().with_context(|| {
                    format!(
                        "failed to parse {} as a network prefix length",
                        prefix_length
                    )
                })?;
                (base, Some(prefix_length))
            }
        };

        let base: IpAddr = base
            .parse()
            .with_context(|| format!("failed to parse {} as a base network IP address", base))?;

        let max_prefix_length = match base {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix_length = prefix_length.unwrap_or(max_prefix_length);
        ensure!(
            prefix_length <= max_prefix_length,
            "invalid network prefix length: {} must be at most {}",
            prefix_length,
            max_prefix_length
        );

        let inner = match base {
            IpAddr::V4(base) => {
                let zero_mask = ((!0) >> prefix_length) << prefix_length;
                NetworkInner::V4 {
                    inclusive_start: Ipv4Addr::from_bits(base.to_bits() & zero_mask),
                    inclusive_end: Ipv4Addr::from_bits(base.to_bits() | !zero_mask),
                }
            }
            IpAddr::V6(base) => {
                let zero_mask = ((!0) >> prefix_length) << prefix_length;
                NetworkInner::V6 {
                    inclusive_start: Ipv6Addr::from_bits(base.to_bits() & zero_mask),
                    inclusive_end: Ipv6Addr::from_bits(base.to_bits() | !zero_mask),
                }
            }
        };

        Ok(Network(inner))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(network: &str, expected_start: &str, expected_end: &str) {
        let network = Network::from_str(network).unwrap();
        let expected_start = IpAddr::from_str(expected_start).unwrap();
        let expected_end = IpAddr::from_str(expected_end).unwrap();
        let (actual_start, actual_end) = match network {
            Network(NetworkInner::V4 {
                inclusive_start,
                inclusive_end,
            }) => (IpAddr::from(inclusive_start), IpAddr::from(inclusive_end)),
            Network(NetworkInner::V6 {
                inclusive_start,
                inclusive_end,
            }) => (IpAddr::from(inclusive_start), IpAddr::from(inclusive_end)),
        };

        assert_eq!(expected_start, actual_start);
        assert_eq!(expected_end, actual_end);
    }

    #[test]
    fn test() {
        check("1.2.3.4", "1.2.3.4", "1.2.3.4");
        check("1.2.3.4/32", "1.2.3.4", "1.2.3.4");
        check("1.2.3.4/30", "1.2.3.4", "1.2.3.7");
        check("255.255.255.255/30", "255.255.255.252", "255.255.255.255");
        check("192.168.1.0/24", "192.168.1.0", "192.168.1.255");

        check(
            "2001:db8:3333:4444:5555:6666:7777:8888",
            "2001:db8:3333:4444:5555:6666:7777:8888",
            "2001:db8:3333:4444:5555:6666:7777:8888",
        );
        check(
            "2001:db8:3333:4444:5555:6666:7777:8888/128",
            "2001:db8:3333:4444:5555:6666:7777:8888",
            "2001:db8:3333:4444:5555:6666:7777:8888",
        );
        check(
            "2001:db8:3333:4400::/56",
            "2001:0db8:3333:4400:0000:0000:0000:0000",
            "2001:0db8:3333:44ff:ffff:ffff:ffff:ffff",
        );
    }

    #[test]
    fn invalid() {
        assert!(Network::from_str("something").is_err());
        assert!(Network::from_str("1000.1.1.1").is_err());
        assert!(Network::from_str("1000.1.1.1/24").is_err());
        assert!(Network::from_str("1.1.1.1/something").is_err());
        assert!(Network::from_str("1.1.1.1/40").is_err());
    }
}
