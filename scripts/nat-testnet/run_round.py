#!/usr/bin/env python3
"""
Run a complete testnet round: provision -> setup -> start -> status -> upload -> logs -> destroy.

Usage:
    python3 run_round.py --name r1-baseline --node-version v0.10.0-rc.13 --layout 60node-80nat
    python3 run_round.py --name r2-quick --node-version v0.10.0-rc.13 --layout 20node-quick --uploads 3 --upload-size 1M
    python3 run_round.py --name r3-test --node-version v0.10.0-rc.13 --layout minimal --skip-destroy

This is the main entry point for reproducible testnet testing. Each round:
  1. Provisions VMs across DO and Hetzner
  2. Sets up binaries + NAT simulation
  3. Starts bootstrap nodes, then all others
  4. Waits for network formation
  5. Checks network status
  6. Runs upload tests
  7. Collects logs
  8. Destroys all VMs (unless --skip-destroy)

The full round state and logs are preserved in state/<name>/ for analysis.
"""

import argparse
import json
import os
import sys
import time

from config import STATE_DIR


def log(msg):
    """Print a timestamped log message."""
    ts = time.strftime("%H:%M:%S")
    print(f"[{ts}] {msg}")


def run_round(args):
    """Execute a complete testnet round."""
    name = args.name
    node_version = args.node_version
    client_version = args.client_version
    layout = args.layout
    upload_count = args.uploads
    upload_size = args.upload_size
    skip_destroy = args.skip_destroy
    skip_upload = args.skip_upload
    skip_logs = args.skip_logs

    round_start = time.time()

    log(f"{'='*60}")
    log(f"TESTNET ROUND: {name}")
    log(f"{'='*60}")
    log(f"  Layout:        {layout}")
    log(f"  Node version:  {node_version}")
    log(f"  Client version:{client_version or '(none)'}")
    log(f"  Uploads:       {upload_count} x {upload_size}")
    log(f"  Skip destroy:  {skip_destroy}")
    log(f"{'='*60}")
    print()

    # --- Phase 1: Provision ---
    log("PHASE 1: Provisioning VMs...")
    phase_start = time.time()
    try:
        from provision import provision
        provision(name, layout, node_version, client_version)
    except Exception as e:
        log(f"PROVISION FAILED: {e}")
        sys.exit(1)
    log(f"Provisioning took {time.time() - phase_start:.0f}s")
    print()

    # --- Phase 2: Setup ---
    log("PHASE 2: Setting up VMs (binaries + NAT)...")
    phase_start = time.time()
    try:
        from setup import setup
        setup_ok = setup(name, node_version, client_version)
    except Exception as e:
        log(f"SETUP FAILED: {e}")
        if not skip_destroy:
            log("Destroying VMs due to setup failure...")
            _destroy_on_failure(name)
        sys.exit(1)
    log(f"Setup took {time.time() - phase_start:.0f}s")
    print()

    # --- Phase 3: Start ---
    log("PHASE 3: Starting nodes...")
    phase_start = time.time()
    try:
        from start import start
        start(name)
    except Exception as e:
        log(f"START FAILED: {e}")
        if not skip_destroy:
            log("Destroying VMs due to start failure...")
            _destroy_on_failure(name)
        sys.exit(1)
    log(f"Start took {time.time() - phase_start:.0f}s")
    print()

    # --- Phase 4: Status ---
    log("PHASE 4: Checking network status...")
    try:
        from status import status as check_status
        summary = check_status(name, verbose=True)
    except Exception as e:
        log(f"STATUS CHECK FAILED: {e}")
        summary = {}
    print()

    # --- Phase 5: Upload ---
    if not skip_upload:
        log(f"PHASE 5: Running {upload_count} upload(s) of {upload_size}...")
        phase_start = time.time()
        try:
            from upload import run_upload
            run_upload(name, upload_count, upload_size)
        except Exception as e:
            log(f"UPLOAD FAILED: {e}")
        log(f"Uploads took {time.time() - phase_start:.0f}s")
        print()
    else:
        log("PHASE 5: Uploads skipped (--skip-upload)")
        print()

    # --- Phase 6: Logs ---
    if not skip_logs:
        log("PHASE 6: Collecting logs...")
        try:
            from logs import collect_logs
            log_dir = collect_logs(name)
        except Exception as e:
            log(f"LOG COLLECTION FAILED: {e}")
            log_dir = None
        print()
    else:
        log("PHASE 6: Log collection skipped (--skip-logs)")
        log_dir = None
        print()

    # --- Phase 7: Destroy ---
    if not skip_destroy:
        log("PHASE 7: Destroying VMs...")
        try:
            from destroy import destroy
            destroy(name, skip_confirm=True)
        except Exception as e:
            log(f"DESTROY FAILED: {e}")
            log("WARNING: VMs may still be running. Check cloud dashboards.")
    else:
        log("PHASE 7: Destroy skipped (--skip-destroy)")
        log("Remember to run: python3 destroy.py --name {name}")
    print()

    # --- Summary ---
    total_time = time.time() - round_start
    log(f"{'='*60}")
    log(f"ROUND COMPLETE: {name}")
    log(f"{'='*60}")
    log(f"  Total time:    {total_time:.0f}s ({total_time/60:.1f}min)")
    if summary:
        log(f"  Processes:     {summary.get('total_procs', '?')}/{summary.get('total_expected', '?')}")
        log(f"  HP success:    {summary.get('hp_ok', 0)}")
        log(f"  HP fail:       {summary.get('hp_fail', 0)}")
        log(f"  Errors:        {summary.get('errors', 0)}")
    if log_dir:
        log(f"  Logs:          {log_dir}")
    log(f"{'='*60}")

    # Save round summary
    _save_round_summary(name, args, summary, total_time, log_dir)


def _destroy_on_failure(testnet_name):
    """Best-effort VM destruction on failure."""
    try:
        from destroy import destroy
        destroy(testnet_name, skip_confirm=True)
    except Exception as e:
        print(f"WARNING: Cleanup also failed: {e}")
        print("Check cloud provider dashboards for orphaned VMs.")


def _save_round_summary(name, args, summary, total_time, log_dir):
    """Save a JSON summary of the round for later comparison."""
    state_dir = os.path.join(STATE_DIR, name)
    os.makedirs(state_dir, exist_ok=True)
    summary_path = os.path.join(state_dir, "round-summary.json")

    data = {
        "name": name,
        "layout": args.layout,
        "node_version": args.node_version,
        "client_version": args.client_version,
        "uploads": args.uploads,
        "upload_size": args.upload_size,
        "total_time_secs": round(total_time, 1),
        "completed_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "network_summary": summary or {},
        "log_dir": log_dir,
    }
    with open(summary_path, "w") as f:
        json.dump(data, f, indent=2)
    print(f"Round summary saved to {summary_path}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Run a complete testnet round",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Full 60-node round with 80% NAT
  python3 run_round.py --name r1-baseline --node-version v0.10.0-rc.13 --layout 60node-80nat

  # Quick smoke test
  python3 run_round.py --name quick-test --node-version v0.10.0-rc.13 --layout 20node-quick

  # Minimal test, keep VMs alive for debugging
  python3 run_round.py --name debug --node-version v0.10.0-rc.13 --layout minimal --skip-destroy
        """,
    )
    parser.add_argument("--name", required=True,
                        help="Round name (used for state dir and VM tags)")
    parser.add_argument("--node-version", required=True,
                        help="ant-node release version (e.g. v0.10.0-rc.13)")
    parser.add_argument("--client-version", default=None,
                        help="ant client release version (for upload tests)")
    parser.add_argument("--layout", required=True,
                        help="Layout name from layouts.py")
    parser.add_argument("--uploads", type=int, default=5,
                        help="Number of upload tests to run (default: 5)")
    parser.add_argument("--upload-size", default="1M",
                        help="Size of each upload test file (default: 1M)")
    parser.add_argument("--skip-destroy", action="store_true",
                        help="Don't destroy VMs after the round")
    parser.add_argument("--skip-upload", action="store_true",
                        help="Skip upload tests")
    parser.add_argument("--skip-logs", action="store_true",
                        help="Skip log collection")
    args = parser.parse_args()

    run_round(args)


if __name__ == "__main__":
    main()
