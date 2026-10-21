//! CIDR membership for the network planes.
//!
//! A node belongs to a plane when its declared address falls inside the
//! CIDR the registry declares for that plane. The comparison is a pure
//! function over the dotted-quad strings the registry carries, so it needs
//! no address-parsing dependency: the plane CIDR is a dotted-quad prefix
//! and the node address is a dotted-quad address.
//!
//! This lives beside `plane_vocabulary` because both answer the same
//! question — "which plane is this node on" — and a caller that has to
//! import two modules to ask one question will import one of them only
//! sometimes.

/// True when `address` falls inside the CIDR block `cidr`.
///
/// Both are IPv4. The comparison is done on the numeric octets.
///
/// A malformed address or CIDR returns `false` rather than an error: the
/// caller is asking "does this address belong to this plane", and an
/// unparseable input is not evidence of membership.
pub fn address_in_cidr(address: &str, cidr: &str) -> bool {
    let Some((network, prefix_len)) = cidr.split_once('/') else {
        return false;
    };
    let Some(prefix_len) = prefix_len.trim().parse::<u32>().ok() else {
        return false;
    };
    if prefix_len > 32 {
        return false;
    }
    let (Some(network_octets), Some(address_octets)) = (
        parse_ipv4_octets(network),
        parse_ipv4_octets(address.trim()),
    ) else {
        return false;
    };
    let mask = if prefix_len == 0 {
        0u32
    } else {
        u32::MAX << (32 - prefix_len)
    };
    let network_value = u32::from_be_bytes(network_octets);
    let address_value = u32::from_be_bytes(address_octets);
    network_value & mask == address_value & mask
}

/// Parse a dotted-quad IPv4 string into its four octets.
fn parse_ipv4_octets(value: &str) -> Option<[u8; 4]> {
    let mut octets = [0u8; 4];
    let mut parts = value.split('.');
    for octet in octets.iter_mut() {
        let part = parts.next()?.trim();
        if part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return None;
        }
        *octet = part.parse::<u8>().ok()?;
    }
    if parts.next().is_some() {
        return None;
    }
    Some(octets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_inside_the_plane_cidr_belongs_to_the_plane() {
        assert!(address_in_cidr("10.10.10.100", "10.10.10.0/24"));
        assert!(address_in_cidr("10.10.10.1", "10.10.10.0/24"));
        assert!(address_in_cidr("10.10.10.255", "10.10.10.0/24"));
    }

    #[test]
    fn an_address_outside_the_plane_cidr_does_not_belong_to_the_plane() {
        assert!(!address_in_cidr("192.168.0.25", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.10.11.1", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.20.0.3", "10.10.10.0/24"));
    }

    #[test]
    fn a_slash_32_matches_only_its_own_address() {
        assert!(address_in_cidr("10.10.10.7", "10.10.10.7/32"));
        assert!(!address_in_cidr("10.10.10.8", "10.10.10.7/32"));
    }

    #[test]
    fn a_slash_0_matches_every_address() {
        assert!(address_in_cidr("192.168.0.25", "0.0.0.0/0"));
        assert!(address_in_cidr("10.10.10.1", "0.0.0.0/0"));
    }

    /// A malformed address or CIDR is not evidence of membership, so it
    /// returns `false` rather than panicking or admitting the node.
    #[test]
    fn a_malformed_address_or_cidr_is_not_membership() {
        assert!(!address_in_cidr("not-an-ip", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.10.10.1", "not-a-cidr"));
        assert!(!address_in_cidr("10.10.10.1", "10.10.10.0"));
        assert!(!address_in_cidr("10.10.10.1", "10.10.10.0/33"));
        assert!(!address_in_cidr("10.10.10.256", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.10.10", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.10.10.1.2", "10.10.10.0/24"));
        assert!(!address_in_cidr("10.10.10.0x1", "10.10.10.0/24"));
    }
}
