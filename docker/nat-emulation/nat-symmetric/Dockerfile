# Symmetric NAT (APDM + APDF)
# - Address+Port Dependent Mapping: Different external port per destination
# - Address+Port Dependent Filtering: Only accepts from exact IP:port we've sent to
#
# The hardest NAT type to traverse. Each connection to a different destination
# gets a completely different external port, making hole-punching nearly impossible.

FROM alpine:3.19

RUN apk add --no-cache \
    iptables \
    iproute2 \
    conntrack-tools \
    bash

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
