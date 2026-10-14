# PyInstaller spec for the standalone cfabric-browser program.
#
# Build from libs/browser, after building the interface (frontend: npm run build),
# in a virtual environment with normal (not editable) installs of libs/core and
# libs/browser[desktop], made with a python.org Python (not the Microsoft Store one):
#
#     pyinstaller packaging/cfabric-browser.spec
#
# The program ends up in dist/cfabric-browser/.

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
]

a = Analysis(
    [str(package / "__main__.py")],
    datas=collect_data_files("cfabric_browser"),
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
    console=True,
)
coll = COLLECT(exe, a.binaries, a.datas, name="cfabric-browser")
