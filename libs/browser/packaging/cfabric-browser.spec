# PyInstaller spec for the standalone cfabric-browser program.
#
# Build from libs/browser, after building the interface (frontend: npm run build),
# in a virtual environment with normal (not editable) installs of libs/core and
# libs/browser[desktop], made with a python.org Python (not the Microsoft Store one):
#
#     pyinstaller packaging/cfabric-browser.spec
#
# The program ends up in dist/cfabric-browser/. It shows the browser in a window of
# its own (pywebview, see cfabric-browser.py); started without corpora, it offers
# the corpora on this computer.

from pathlib import Path

from PyInstaller.utils.hooks import collect_data_files, collect_submodules

import cfabric_browser

package = Path(cfabric_browser.__file__).parent
if not (package / "static" / "index.html").exists():
    raise SystemExit(
        f"No built interface in {package / 'static'}: run npm run build in frontend/"
    )

hiddenimports = [
    *collect_submodules("cfabric"),
    *collect_submodules("cfabric_browser"),
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
)
coll = COLLECT(exe, a.binaries, a.datas, name="cfabric-browser")
