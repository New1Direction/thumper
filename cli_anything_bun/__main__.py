"""
Entry point for `python -m cli_anything_bun`.

This allows running the harness directly without installing it as a package:

    python -m cli_anything_bun script run dev
    python -m cli_anything_bun package add hono --dev
"""

from .cli import main

if __name__ == "__main__":
    import sys
    sys.exit(main())
