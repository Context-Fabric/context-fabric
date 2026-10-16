"""Pack the Claude Desktop extension: `Context-Fabric.mcpb`, a zip with the
manifest at its top.

    python packaging/mcpb/pack.py OUT.mcpb
"""

import json
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).parent
FILES = ["manifest.json", "server/index.js"]


def pack(out: Path) -> Path:
    json.loads((HERE / "manifest.json").read_text(encoding="utf-8"))  # valid JSON
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as archive:
        for name in FILES:
            archive.write(HERE / name, name)
    return out


if __name__ == "__main__":
    print(pack(Path(sys.argv[1] if len(sys.argv) > 1 else "Context-Fabric.mcpb")))
