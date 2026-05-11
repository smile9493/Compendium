#!/usr/bin/env python3
"""
AI-native PDF knowledge compilation engine - MCP server
"""

import os
import sys
import subprocess
import platform
from pathlib import Path


def get_binary_name() -> str:
    """Get the binary name for the current platform."""
    system = platform.system().lower()
    machine = platform.machine().lower()
    
    if system == "windows":
        return "pdf-mcp.exe"
    elif system == "darwin":
        return "pdf-mcp-macos-arm64" if machine == "arm64" else "pdf-mcp-macos-x64"
    else:
        return "pdf-mcp-linux-x64"


def get_binary_path() -> Path:
    """Get the path to the binary for the current platform."""
    binary_name = get_binary_name()
    package_dir = Path(__file__).parent
    return package_dir / "binaries" / binary_name


def main():
    """Main entry point."""
    binary_path = get_binary_path()
    
    if not binary_path.exists():
        print(f"Binary not found for platform: {platform.system()}-{platform.machine()}", file=sys.stderr)
        print(f"Expected path: {binary_path}", file=sys.stderr)
        sys.exit(1)
    
    env = os.environ.copy()
    if "PDFIUM_LIB_PATH" not in env:
        package_dir = Path(__file__).parent
        pdfium_path = package_dir / "binaries" / "libpdfium.so"
        if pdfium_path.exists():
            env["PDFIUM_LIB_PATH"] = str(pdfium_path)
    
    result = subprocess.run(
        [str(binary_path)] + sys.argv[1:],
        env=env,
    )
    sys.exit(result.returncode)


if __name__ == "__main__":
    main()
