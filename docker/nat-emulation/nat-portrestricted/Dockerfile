# Port-Restricted NAT (EIM + APDF)
# - Endpoint Independent Mapping: Same external port for all destinations
# - Address+Port Dependent Filtering: Only accepts from exact IP:port we've sent to
#
# The most common home router NAT type. External hosts can only send back
# if we've sent to their exact IP:port combination.

FROM alpine:3.19

RUN apk add --no-cache \
    iptables \
    iproute2 \
    conntrack-tools \
    bash

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
