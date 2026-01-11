# Full Cone NAT (EIM + EIF)
# - Endpoint Independent Mapping: Same external port for all destinations
# - Endpoint Independent Filtering: Accepts packets from any external host
#
# This is the most permissive NAT type. Once an internal host sends a packet
# out, any external host can send packets back to that mapped port.

FROM alpine:3.19

RUN apk add --no-cache \
    iptables \
    iproute2 \
    conntrack-tools \
    bash

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
