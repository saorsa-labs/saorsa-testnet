//! Bootstrap peer configuration for gossip-first peer discovery.
//!
//! This module provides the hardcoded list of VPS nodes that serve as bootstrap
//! peers for the network. Unlike the registry-based approach, this uses direct
//! IP addresses to ensure reliable connectivity.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Gossip-First Discovery                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                  │
//! │   1. Node starts with hardcoded VPS bootstrap peers             │
//! │   2. Connects to any available bootstrap peer                   │
//! │   3. Receives full peer cache via gossip sync                   │
//! │   4. Broadcasts own presence via epidemic gossip                │
//! │   5. saorsa-1 acts as relay/coordinator (not registry)          │
//! │                                                                  │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # VPS Node Roles
//!
//! - **saorsa-1**: Relay server and NAT traversal coordinator (public-1)
//! - **saorsa-2 to saorsa-9**: Bootstrap peers with NAT simulation
//! - **saorsa-10**: Backup relay (public-2)
//!
//! # NAT Simulation Types
//!
//! Each node simulates a specific NAT type for testing connectivity:
//! - **Public**: No NAT (saorsa-1, saorsa-10)
//! - **Full Cone**: Most permissive (saorsa-2, saorsa-3)
//! - **Address Restricted**: Medium difficulty (saorsa-4, saorsa-5)
//! - **Port Restricted**: Hard (saorsa-6, saorsa-7)
//! - **Symmetric**: Most restrictive (saorsa-8, saorsa-9)

use crate::NatType;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// A bootstrap peer with its network addresses and capabilities.
#[derive(Debug, Clone)]
pub struct BootstrapPeer {
    /// Human-readable name (e.g., "saorsa-2")
    pub name: &'static str,
    /// DNS name for identification (e.g., "full-cone-1.saorsalabs.com")
    pub dns_name: &'static str,
    /// IPv4 address
    pub ipv4: Ipv4Addr,
    /// IPv6 address (if available)
    pub ipv6: Option<Ipv6Addr>,
    /// P2P port for QUIC connections
    pub port: u16,
    /// Whether this node supports relay functionality
    pub is_relay: bool,
    /// Whether this node supports NAT coordination
    pub is_coordinator: bool,
    /// Simulated NAT type for testing
    pub nat_type: NatType,
}

impl BootstrapPeer {
    /// Get the IPv4 socket address.
    #[must_use]
    pub fn socket_addr_v4(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(self.ipv4), self.port)
    }

    /// Get the IPv6 socket address, if available.
    #[must_use]
    pub fn socket_addr_v6(&self) -> Option<SocketAddr> {
        self.ipv6
            .map(|ip| SocketAddr::new(IpAddr::V6(ip), self.port))
    }

    /// Get all socket addresses (IPv4 and optionally IPv6).
    #[must_use]
    pub fn all_addrs(&self) -> Vec<SocketAddr> {
        let mut addrs = vec![self.socket_addr_v4()];
        if let Some(v6) = self.socket_addr_v6() {
            addrs.push(v6);
        }
        addrs
    }
}

/// Hardcoded VPS bootstrap peers with their actual IP addresses.
///
/// These are the known VPS nodes that form the backbone of the test network.
/// Each node is pre-configured and always available for bootstrapping.
///
/// # NAT Type Distribution
///
/// | DNS Name | NAT Type | Hole-Punch Difficulty |
/// |----------|----------|----------------------|
/// | saorsa-1.saorsalabs.com | Public | Direct |
/// | full-cone-1/2 | Full Cone | Easy |
/// | addr-rest-1/2 | Address Restricted | Medium |
/// | port-rest-1/2 | Port Restricted | Hard |
/// | symmetric-1/2 | Symmetric | Very Hard |
/// | public-2 | Public | Direct |
pub const BOOTSTRAP_PEERS: &[BootstrapPeer] = &[
    // saorsa-1: Registry/Relay/Coordinator (primary infrastructure)
    // DNS: saorsa-1.saorsalabs.com (kept for TLS certificate)
    BootstrapPeer {
        name: "saorsa-1",
        dns_name: "saorsa-1.saorsalabs.com",
        ipv4: Ipv4Addr::new(77, 42, 75, 115),
        ipv6: None,
        port: 9000,
        is_relay: true,
        is_coordinator: true,
        nat_type: NatType::None,
    },
    // saorsa-2: DigitalOcean NYC - Full Cone NAT simulation
    // DNS: full-cone-1.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-2",
        dns_name: "full-cone-1.saorsalabs.com",
        ipv4: Ipv4Addr::new(142, 93, 199, 50),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::FullCone,
    },
    // saorsa-3: DigitalOcean NYC - Full Cone NAT simulation
    // DNS: full-cone-2.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-3",
        dns_name: "full-cone-2.saorsalabs.com",
        ipv4: Ipv4Addr::new(147, 182, 234, 192),
        ipv6: Some(Ipv6Addr::new(
            0x2604, 0xa880, 0x0004, 0x01d0, 0, 0x0001, 0x6ba1, 0xf000,
        )),
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::FullCone,
    },
    // saorsa-4: DigitalOcean SFO - Address Restricted NAT simulation
    // DNS: addr-rest-1.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-4",
        dns_name: "addr-rest-1.saorsalabs.com",
        ipv4: Ipv4Addr::new(206, 189, 7, 117),
        ipv6: Some(Ipv6Addr::new(
            0x2a03, 0xb0c0, 0x0002, 0x00f0, 0, 0x0001, 0x26a1, 0x8001,
        )),
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::AddressRestricted,
    },
    // saorsa-5: DigitalOcean LON - Address Restricted NAT simulation
    // DNS: addr-rest-2.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-5",
        dns_name: "addr-rest-2.saorsalabs.com",
        ipv4: Ipv4Addr::new(144, 126, 230, 161),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::AddressRestricted,
    },
    // saorsa-6: Hetzner FIN - Port Restricted NAT simulation
    // DNS: port-rest-1.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-6",
        dns_name: "port-rest-1.saorsalabs.com",
        ipv4: Ipv4Addr::new(65, 21, 157, 229),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::PortRestricted,
    },
    // saorsa-7: Hetzner DE - Port Restricted NAT simulation
    // DNS: port-rest-2.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-7",
        dns_name: "port-rest-2.saorsalabs.com",
        ipv4: Ipv4Addr::new(116, 203, 101, 172),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::PortRestricted,
    },
    // saorsa-8: Vultr Tokyo - Symmetric NAT simulation
    // DNS: symmetric-1.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-8",
        dns_name: "symmetric-1.saorsalabs.com",
        ipv4: Ipv4Addr::new(149, 28, 156, 231),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::Symmetric,
    },
    // saorsa-9: Vultr Miami - Symmetric NAT simulation
    // DNS: symmetric-2.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-9",
        dns_name: "symmetric-2.saorsalabs.com",
        ipv4: Ipv4Addr::new(45, 77, 176, 184),
        ipv6: None,
        port: 9000,
        is_relay: false,
        is_coordinator: true,
        nat_type: NatType::Symmetric,
    },
    // saorsa-10: Backup relay - Public IP
    // DNS: public-2.saorsalabs.com
    BootstrapPeer {
        name: "saorsa-10",
        dns_name: "public-2.saorsalabs.com",
        ipv4: Ipv4Addr::new(77, 42, 39, 239),
        ipv6: None,
        port: 9000,
        is_relay: true,
        is_coordinator: true,
        nat_type: NatType::None,
    },
];

/// Get all bootstrap peer IPv4 addresses.
#[must_use]
pub fn bootstrap_addrs_v4() -> Vec<SocketAddr> {
    BOOTSTRAP_PEERS
        .iter()
        .map(BootstrapPeer::socket_addr_v4)
        .collect()
}

/// Get all bootstrap peer addresses (both IPv4 and IPv6).
#[must_use]
pub fn bootstrap_addrs_all() -> Vec<SocketAddr> {
    BOOTSTRAP_PEERS
        .iter()
        .flat_map(BootstrapPeer::all_addrs)
        .collect()
}

/// Get the primary relay node (saorsa-1).
#[must_use]
pub fn relay_node() -> &'static BootstrapPeer {
    &BOOTSTRAP_PEERS[0]
}

/// Get all coordinator nodes.
#[must_use]
pub fn coordinator_nodes() -> Vec<&'static BootstrapPeer> {
    BOOTSTRAP_PEERS
        .iter()
        .filter(|p| p.is_coordinator)
        .collect()
}

/// Check if an IP address belongs to a known VPS node.
#[must_use]
pub fn is_vps_ip(addr: &IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => BOOTSTRAP_PEERS.iter().any(|p| &p.ipv4 == v4),
        IpAddr::V6(v6) => BOOTSTRAP_PEERS.iter().any(|p| p.ipv6.as_ref() == Some(v6)),
    }
}

/// Check if a socket address belongs to a known VPS node.
#[must_use]
pub fn is_vps_addr(addr: &SocketAddr) -> bool {
    is_vps_ip(&addr.ip())
}

/// Get the VPS peer by name.
#[must_use]
pub fn get_peer_by_name(name: &str) -> Option<&'static BootstrapPeer> {
    BOOTSTRAP_PEERS.iter().find(|p| p.name == name)
}

/// Get the VPS peer by DNS name.
#[must_use]
pub fn get_peer_by_dns(dns_name: &str) -> Option<&'static BootstrapPeer> {
    BOOTSTRAP_PEERS.iter().find(|p| p.dns_name == dns_name)
}

/// Get all peers with a specific NAT type.
#[must_use]
pub fn peers_by_nat_type(nat_type: NatType) -> Vec<&'static BootstrapPeer> {
    BOOTSTRAP_PEERS
        .iter()
        .filter(|p| p.nat_type == nat_type)
        .collect()
}

/// Get the NAT type for a known VPS IP address.
#[must_use]
pub fn nat_type_for_ip(addr: &IpAddr) -> Option<NatType> {
    match addr {
        IpAddr::V4(v4) => BOOTSTRAP_PEERS
            .iter()
            .find(|p| &p.ipv4 == v4)
            .map(|p| p.nat_type),
        IpAddr::V6(v6) => BOOTSTRAP_PEERS
            .iter()
            .find(|p| p.ipv6.as_ref() == Some(v6))
            .map(|p| p.nat_type),
    }
}

/// Get the DNS name for a known VPS IP address.
#[must_use]
pub fn dns_name_for_ip(addr: &IpAddr) -> Option<&'static str> {
    match addr {
        IpAddr::V4(v4) => BOOTSTRAP_PEERS
            .iter()
            .find(|p| &p.ipv4 == v4)
            .map(|p| p.dns_name),
        IpAddr::V6(v6) => BOOTSTRAP_PEERS
            .iter()
            .find(|p| p.ipv6.as_ref() == Some(v6))
            .map(|p| p.dns_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_peers_count() {
        assert_eq!(
            BOOTSTRAP_PEERS.len(),
            10,
            "Should have 10 VPS bootstrap peers"
        );
    }

    #[test]
    fn test_relay_node() {
        let relay = relay_node();
        assert_eq!(relay.name, "saorsa-1");
        assert!(relay.is_relay);
    }

    #[test]
    fn test_vps_ip_detection() {
        let saorsa1_ip = IpAddr::V4(Ipv4Addr::new(77, 42, 75, 115));
        assert!(is_vps_ip(&saorsa1_ip));

        let random_ip = IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4));
        assert!(!is_vps_ip(&random_ip));
    }

    #[test]
    fn test_bootstrap_addrs_v4() {
        let addrs = bootstrap_addrs_v4();
        assert_eq!(addrs.len(), 9);
        assert!(addrs.iter().all(|a| a.port() == 9000));
    }

    #[test]
    fn test_coordinator_nodes() {
        let coordinators = coordinator_nodes();
        // All VPS nodes are coordinators
        assert_eq!(coordinators.len(), 9);
    }

    #[test]
    fn test_peer_by_name() {
        let peer = get_peer_by_name("saorsa-5");
        assert!(peer.is_some());
        assert_eq!(peer.unwrap().ipv4, Ipv4Addr::new(144, 126, 230, 161));
    }
}
