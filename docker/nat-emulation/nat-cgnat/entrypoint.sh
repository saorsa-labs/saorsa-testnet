#!/bin/bash
# CGNAT entrypoint
# Carrier-Grade NAT simulation with limited port range

set -e

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Limit local port range (simulates CGNAT port exhaustion scenarios)
# Only 256 ports available (32768-33023)
echo "32768 33023" > /proc/sys/net/ipv4/ip_local_port_range

# Flush existing rules
iptables -F
iptables -t nat -F

# CGNAT: SNAT with limited port range
# Using our external IP with restricted port range
EXTERNAL_IP=$(ip -4 addr show eth0 | grep -oP '(?<=inet\s)\d+(\.\d+){3}')
iptables -t nat -A POSTROUTING -o eth0 -p udp -j SNAT --to-source ${EXTERNAL_IP}:32768-33023
iptables -t nat -A POSTROUTING -o eth0 -p tcp -j SNAT --to-source ${EXTERNAL_IP}:32768-33023
# Fallback for other protocols
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE

# Default policy: DROP forwarded packets
iptables -P FORWARD DROP

# Allow only ESTABLISHED connections
iptables -A FORWARD -m conntrack --ctstate ESTABLISHED -j ACCEPT

# Allow outbound from internal
iptables -A FORWARD -i eth1 -o eth0 -j ACCEPT

echo "CGNAT configured"
echo "External interface: eth0"
echo "External IP: ${EXTERNAL_IP}"
echo "Port range: 32768-33023 (256 ports)"
echo "Filtering: Address+Port dependent"

# Show rules for debugging
echo ""
echo "NAT rules:"
iptables -t nat -L -n -v
echo ""
echo "Filter rules:"
iptables -L -n -v
echo ""
echo "Port range:"
cat /proc/sys/net/ipv4/ip_local_port_range

# Keep container running
exec tail -f /dev/null
