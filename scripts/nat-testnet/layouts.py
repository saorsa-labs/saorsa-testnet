"""
Predefined testnet layouts.

Each layout defines how many VMs of each role to create per cloud provider,
and how many ant-node processes to run on each VM.

Roles:
  - bootstrap : Public node, started first, other nodes connect to it
  - public    : Public node (no NAT)
  - nat       : Node behind port-restricted NAT simulation
  - client    : VM used for running upload/download tests (ant CLI)
"""

LAYOUTS = {
    # -------------------------------------------------------------------
    # 60-node network, ~80% behind port-restricted NAT
    # DO: 4 bootstrap + 10 NAT + 1 client = 15 droplets
    # HC: 3 public + 2 NAT = 5 servers (hits Hetzner limit)
    # Total nodes: (4+10+3+2) * 2 = 38 node-processes (with nodes_per_vm=2)
    # NAT ratio: (10+2)*2 / (4+10+3+2)*2 = 24/38 = 63%
    # For higher NAT ratio, increase DO nat count.
    # -------------------------------------------------------------------
    "60node-80nat": {
        "do": {"bootstrap": 4, "nat": 10, "client": 1},
        "hc": {"public": 3, "nat": 2},
        "nodes_per_vm": 2,
        "description": "Large testnet: ~60 nodes, ~80% port-restricted NAT",
    },

    # -------------------------------------------------------------------
    # Quick smoke-test layout
    # DO: 2 bootstrap + 4 NAT + 1 client = 7 droplets
    # HC: 1 public + 0 NAT = 1 server
    # Total nodes: (2+4+1)*2 = 14 node-processes
    # -------------------------------------------------------------------
    "20node-quick": {
        "do": {"bootstrap": 2, "nat": 4, "client": 1},
        "hc": {"public": 1, "nat": 0},
        "nodes_per_vm": 2,
        "description": "Quick smoke test: ~20 nodes, moderate NAT",
    },

    # -------------------------------------------------------------------
    # Minimal: just enough to test hole-punching
    # -------------------------------------------------------------------
    "minimal": {
        "do": {"bootstrap": 1, "nat": 2, "client": 1},
        "hc": {"public": 1, "nat": 0},
        "nodes_per_vm": 1,
        "description": "Minimal: 4 nodes, basic hole-punch test",
    },
}


def get_layout(name):
    """Return a layout dict by name, or exit with an error."""
    layout = LAYOUTS.get(name)
    if layout is None:
        available = ", ".join(sorted(LAYOUTS.keys()))
        print(f"ERROR: Unknown layout '{name}'. Available: {available}")
        raise SystemExit(1)
    return layout


def describe_layout(name):
    """Print a human-readable summary of a layout."""
    layout = get_layout(name)
    npv = layout["nodes_per_vm"]
    do_cfg = layout.get("do", {})
    hc_cfg = layout.get("hc", {})

    do_vms = sum(do_cfg.values())
    hc_vms = sum(hc_cfg.values())
    total_vms = do_vms + hc_vms

    # Nodes run on everything except client VMs
    do_node_vms = do_vms - do_cfg.get("client", 0)
    hc_node_vms = hc_vms - hc_cfg.get("client", 0)
    total_nodes = (do_node_vms + hc_node_vms) * npv

    nat_vms = do_cfg.get("nat", 0) + hc_cfg.get("nat", 0)
    nat_nodes = nat_vms * npv
    nat_pct = (nat_nodes / total_nodes * 100) if total_nodes > 0 else 0

    print(f"Layout: {name}")
    print(f"  {layout.get('description', '')}")
    print(f"  DO VMs: {do_vms} (bootstrap={do_cfg.get('bootstrap', 0)}, "
          f"nat={do_cfg.get('nat', 0)}, client={do_cfg.get('client', 0)})")
    print(f"  HC VMs: {hc_vms} (public={hc_cfg.get('public', 0)}, "
          f"nat={hc_cfg.get('nat', 0)})")
    print(f"  Total VMs: {total_vms}")
    print(f"  Nodes per VM: {npv}")
    print(f"  Total nodes: {total_nodes}")
    print(f"  NAT nodes: {nat_nodes} ({nat_pct:.0f}%)")
