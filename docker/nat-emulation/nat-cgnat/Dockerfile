# Carrier-Grade NAT (CGNAT)
# Simulates ISP-level NAT with limited port range
#
# CGNAT uses RFC 6598 shared address space (100.64.0.0/10) but for our
# simulation we use private addresses. The key characteristic is the
# limited port range (256 ports) that can cause port exhaustion.

FROM alpine:3.19

RUN apk add --no-cache \
    iptables \
    iproute2 \
    conntrack-tools \
    bash

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
