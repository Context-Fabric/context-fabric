# PyInstaller spec for the standalone cfabric-browser program.
#
# Build from libs/browser, after building the interface (frontend: npm run build),
# in a virtual environment with normal (not editable) installs of libs/core and
# libs/browser[desktop], made with a python.org Python (not the Microsoft Store one):
#
#     pyinstaller packaging/cfabric-browser.spec
#
# The program ends up in dist/cfabric-browser/ (on macOS also dist/Context-Fabric.app). It shows the browser in a window of
# its own (pywebview, see cfabric-browser.py); started without corpora, it offers
# the corpora on this computer.

import sys
from pathlib import Path

from PyInstaller.utils.hooks import collect_data_files, collect_submodules

import cfabric_browser

package = Path(cfabric_browser.__file__).parent
icons = Path(SPECPATH) / "icons"  # made by make_icons.py
if not (package / "static" / "index.html").exists():
    raise SystemExit(
        f"No built interface in {package / 'static'}: run npm run build in frontend/"
    )

hiddenimports = [
    *collect_submodules("cfabric"),
    *collect_submodules("cfabric_browser"),
    # searching in plain language: the MCP tools and server (not mcp's own CLI)
    *collect_submodules("cfabric_mcp"),
    *collect_submodules("mcp", filter=lambda name: not name.startswith("mcp.cli")),
    *collect_submodules("uvicorn"),
    *collect_submodules("webview"),
]

a = Analysis(
    [str(Path(SPECPATH) / "cfabric-browser.py")],
    datas=[*collect_data_files("cfabric_browser"), *collect_data_files("webview")],
    hiddenimports=hiddenimports,
    excludes=["tkinter"],
)
pyz = PYZ(a.pure)
exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="cfabric-browser",
    console=False,  # a window, no console
    icon=str(icons / "cfabric-browser.ico"),
)
coll = COLLECT(exe, a.binaries, a.datas, name="cfabric-browser")
if sys.platform == "darwin":
    # a double-clickable application, dist/Context-Fabric.app
    app = BUNDLE(
        coll,
        name="Context-Fabric.app",
        bundle_identifier="ai.context-fabric.browser",
        icon=str(icons / "cfabric-browser.icns"),
        version=cfabric_browser.__version__,
    )
