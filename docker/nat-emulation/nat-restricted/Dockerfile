# Address-Restricted NAT (EIM + ADF)
# - Endpoint Independent Mapping: Same external port for all destinations
# - Address Dependent Filtering: Only accepts from IPs we've sent to
#
# An external host can send to the mapped port only if the internal host
# has previously sent a packet to that external host's IP address.

FROM alpine:3.19

RUN apk add --no-cache \
    iptables \
    iproute2 \
    conntrack-tools \
    bash

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
