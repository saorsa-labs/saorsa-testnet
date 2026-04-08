"""
SSH utility functions using Python subprocess.

All remote commands go through subprocess.run() -- never bash while-read
loops or os.system(), which have shell environment issues with cargo aliases.
"""

import os
import subprocess
import sys
import time

from config import SSH_KEY_PATH


def _ssh_key():
    """Return the expanded SSH key path."""
    return os.path.expanduser(SSH_KEY_PATH)


def _ssh_base_args(ip, user="root"):
    """Return the base SSH argument list."""
    return [
        "ssh",
        "-o", "ConnectTimeout=15",
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "LogLevel=ERROR",
        "-i", _ssh_key(),
        f"{user}@{ip}",
    ]


def ssh_run(ip, command, user="root", timeout=120, check=True, capture=True):
    """
    Run a command on a remote host via SSH.

    Returns subprocess.CompletedProcess.
    Raises subprocess.CalledProcessError if check=True and the command fails.
    """
    args = _ssh_base_args(ip, user) + [command]
    result = subprocess.run(
        args,
        capture_output=capture,
        text=True,
        timeout=timeout,
    )
    if check and result.returncode != 0:
        stderr_snippet = (result.stderr or "")[:500]
        print(f"  SSH command failed on {ip}: {command}", file=sys.stderr)
        if stderr_snippet:
            print(f"  stderr: {stderr_snippet}", file=sys.stderr)
        result.check_returncode()
    return result


def ssh_run_quiet(ip, command, user="root", timeout=120):
    """
    Run a command on a remote host via SSH, ignoring errors.
    Returns (returncode, stdout, stderr).
    """
    try:
        result = ssh_run(ip, command, user=user, timeout=timeout, check=False)
        return result.returncode, result.stdout or "", result.stderr or ""
    except subprocess.TimeoutExpired:
        return -1, "", "timeout"
    except Exception as e:
        return -1, "", str(e)


def scp_to(ip, local_path, remote_path, user="root"):
    """Copy a local file to a remote host."""
    args = [
        "scp",
        "-o", "ConnectTimeout=15",
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "LogLevel=ERROR",
        "-i", _ssh_key(),
        local_path,
        f"{user}@{ip}:{remote_path}",
    ]
    subprocess.run(args, check=True, capture_output=True, text=True, timeout=120)


def scp_from(ip, remote_path, local_path, user="root"):
    """Copy a remote file to local."""
    args = [
        "scp",
        "-o", "ConnectTimeout=15",
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "LogLevel=ERROR",
        "-i", _ssh_key(),
        f"{user}@{ip}:{remote_path}",
        local_path,
    ]
    subprocess.run(args, check=True, capture_output=True, text=True, timeout=300)


def write_remote_file(ip, remote_path, content, user="root"):
    """Write content to a file on a remote host via SSH stdin."""
    args = _ssh_base_args(ip, user) + [f"cat > {remote_path}"]
    subprocess.run(
        args,
        input=content,
        check=True,
        capture_output=True,
        text=True,
        timeout=30,
    )


def wait_for_ssh(ip, retries=20, delay=10):
    """
    Wait until SSH is available on a host.
    Returns True if SSH is reachable, False otherwise.
    """
    for attempt in range(1, retries + 1):
        rc, stdout, _ = ssh_run_quiet(ip, "echo ok", timeout=10)
        if rc == 0 and "ok" in stdout:
            return True
        if attempt < retries:
            print(f"  Waiting for SSH on {ip} (attempt {attempt}/{retries})...")
            time.sleep(delay)
    return False
