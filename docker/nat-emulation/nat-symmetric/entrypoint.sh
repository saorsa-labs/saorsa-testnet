#!/bin/bash
# Symmetric NAT entrypoint
# RFC 4787 classification: APDM (Address+Port Dependent Mapping) + APDF (Address+Port Dependent Filtering)

set -e

# Enable IP forwarding
echo 1 > /proc/sys/net/ipv4/ip_forward

# Flush existing rules
iptables -F
iptables -t nat -F

# Symmetric NAT: Use --random-fully for random port allocation per connection
# This creates different external ports for each destination
iptables -t nat -A POSTROUTING -o eth0 -j MASQUERADE --random-fully

# Default policy: DROP forwarded packets
iptables -P FORWARD DROP

# Allow only ESTABLISHED connections - strict filtering
iptables -A FORWARD -m conntrack --ctstate ESTABLISHED -j ACCEPT

# Allow outbound from internal
iptables -A FORWARD -i eth1 -o eth0 -j ACCEPT

echo "Symmetric NAT configured"
echo "External interface: eth0"
echo "Internal interface: eth1"
echo "Mapping: Random port per destination (--random-fully)"
echo "Filtering: Address+Port dependent"

# Show rules for debugging
echo ""
echo "NAT rules:"
iptables -t nat -L -n -v
echo ""
echo "Filter rules:"
iptables -L -n -v

# Keep container running
exec tail -f /dev/null
