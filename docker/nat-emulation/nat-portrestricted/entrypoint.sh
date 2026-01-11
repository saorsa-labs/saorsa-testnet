#!/bin/bash
# Port-Restricted NAT entrypoint
# RFC 4787 classification: EIM (Endpoint Independent Mapping) + APDF (Address+Port Dependent Filtering)

set -e

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing rules
iptables -F
iptables -t nat -F

# Port-Restricted NAT (most common home router behavior)
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE

# Default policy: DROP forwarded packets
iptables -P FORWARD DROP

# Allow only ESTABLISHED connections - this enforces port-restricted behavior
# RELATED is not included to be more strict (true port-restricted)
iptables -A FORWARD -m conntrack --ctstate ESTABLISHED -j ACCEPT

# Allow outbound from internal
iptables -A FORWARD -i eth1 -o eth0 -j ACCEPT

echo "Port-Restricted NAT configured"
echo "External interface: eth0"
echo "Internal interface: eth1"
echo "Filtering: Address+Port dependent (must have sent to exact IP:port)"

# Show rules for debugging
echo ""
echo "NAT rules:"
iptables -t nat -L -n -v
echo ""
echo "Filter rules:"
iptables -L -n -v

# Keep container running
exec tail -f /dev/null
