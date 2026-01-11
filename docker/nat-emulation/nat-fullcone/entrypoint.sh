#!/bin/bash
# Full Cone NAT entrypoint
# RFC 4787 classification: EIM (Endpoint Independent Mapping) + EIF (Endpoint Independent Filtering)

set -e

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing rules
iptables -F
iptables -t nat -F

# Full Cone NAT: MASQUERADE with open forwarding
# The key difference is we ACCEPT all forwarded traffic, not just ESTABLISHED
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE

# Allow ALL forwarding in both directions (Full Cone behavior)
iptables -A FORWARD -i eth0 -o eth1 -j ACCEPT
iptables -A FORWARD -i eth1 -o eth0 -j ACCEPT

echo "Full Cone NAT configured"
echo "External interface: eth0"
echo "Internal interface: eth1"
echo "Forwarding: OPEN (any external host can reach mapped ports)"

# Show rules for debugging
echo ""
echo "NAT rules:"
iptables -t nat -L -n -v
echo ""
echo "Filter rules:"
iptables -L -n -v

# Keep container running
exec tail -f /dev/null
