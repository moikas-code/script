#!/usr/bin/env python3
"""
Command-line tool for fixing Rust format issues in the Script project.

Usage:
    python tools/fix_rust_format.py analyze     # Check for issues
    python tools/fix_rust_format.py fix          # Fix all issues
    python tools/fix_rust_format.py fix -f path/to/file.rs  # Fix specific file
"""

import sys
import os

# Add parent directory to path to import devutils
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from devutils.rust_format_fixer import main

if __name__ == "__main__":
    main()