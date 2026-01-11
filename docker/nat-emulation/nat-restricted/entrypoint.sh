#!/bin/bash
# Address-Restricted NAT entrypoint
# RFC 4787 classification: EIM (Endpoint Independent Mapping) + ADF (Address Dependent Filtering)

set -e

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing rules
iptables -F
iptables -t nat -F

# Address-Restricted NAT
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE

# Default policy: DROP forwarded packets
iptables -P FORWARD DROP

# Allow established/related connections (standard NAT behavior)
iptables -A FORWARD -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

# Allow outbound from internal
iptables -A FORWARD -i eth1 -o eth0 -j ACCEPT

# The restriction happens through conntrack - packets from IPs we haven't
# contacted will be dropped because they won't match ESTABLISHED/RELATED

echo "Address-Restricted NAT configured"
echo "External interface: eth0"
echo "Internal interface: eth1"
echo "Filtering: Address-dependent (must have previously contacted IP)"

# Show rules for debugging
echo ""
echo "NAT rules:"
iptables -t nat -L -n -v
echo ""
echo "Filter rules:"
iptables -L -n -v

# Keep container running
exec tail -f /dev/null
