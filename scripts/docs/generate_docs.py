#!/usr/bin/env python3
"""Generate JSON API documentation for all Context-Fabric packages.

This script uses Griffe to extract API documentation from Python source code
and outputs structured JSON files suitable for consumption by a Next.js website.

Usage:
    python scripts/docs/generate_docs.py

Output:
    docs/api/index.json           - Combined metadata and navigation
    docs/api/cfabric.json         - Core package API
    docs/api/cfabric_mcp.json     - MCP package API
    docs/api/cfabric_benchmarks.json - Benchmarks package API
"""

from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

# Configuration
PACKAGES = [
    ("cfabric", "libs/core"),
    ("cfabric_mcp", "libs/mcp"),
    ("cfabric_benchmarks", "libs/benchmarks"),
]

REPO_ROOT = Path(__file__).parent.parent.parent
OUTPUT_DIR = REPO_ROOT / "docs" / "api"


def generate_package_docs(package_name: str, package_path: str) -> dict[str, Any]:
    """Generate Griffe JSON for a package.

    Parameters
    ----------
    package_name : str
        The Python package name to document.
    package_path : str
        Path to the package directory relative to repo root.

    Returns
    -------
    dict
        Parsed JSON output from Griffe.

    Raises
    ------
    RuntimeError
        If Griffe fails to generate documentation.
    """
    result = subprocess.run(
        [
            sys.executable, "-m", "griffe", "dump", package_name,
            "--docstyle", "numpy",
            "--search", str(REPO_ROOT / package_path),
            "--resolve-aliases",
        ],
        capture_output=True,
        text=True,
        cwd=REPO_ROOT,
    )

    if result.returncode != 0:
        raise RuntimeError(f"Griffe failed for {package_name}: {result.stderr}")

    return json.loads(result.stdout)


def process_docstring(docstring: dict[str, Any] | None) -> dict[str, Any]:
    """Parse docstring into structured format.

    Parameters
    ----------
    docstring : dict | None
        Griffe docstring object.

    Returns
    -------
    dict
        Structured docstring with summary, description, and sections.
    """
    if not docstring:
        return {"summary": "", "description": "", "sections": {}}

    value = docstring.get("value", "")
    parts = value.split("\n\n", 1)

    return {
        "summary": parts[0].strip() if parts else "",
        "description": value,
        "parsed": docstring.get("parsed", []),
    }


def annotation_to_string(annotation: Any) -> str:
    """Convert a Griffe annotation expression to a string.

    Parameters
    ----------
    annotation : Any
        Griffe annotation object (can be dict, str, or None).

    Returns
    -------
    str
        String representation of the type annotation.
    """
    if annotation is None:
        return ""
    if isinstance(annotation, str):
        return annotation
    if not isinstance(annotation, dict):
        return str(annotation)

    # Handle different expression classes
    cls = annotation.get("cls", "")

    if cls == "ExprName":
        return annotation.get("name", "")

    if cls == "ExprBinOp":
        left = annotation_to_string(annotation.get("left"))
        right = annotation_to_string(annotation.get("right"))
        op = annotation.get("operator", "|")
        return f"{left} {op} {right}"

    if cls == "ExprSubscript":
        left = annotation_to_string(annotation.get("left"))
        slice_val = annotation_to_string(annotation.get("slice"))
        return f"{left}[{slice_val}]"

    if cls == "ExprTuple":
        elements = annotation.get("elements", [])
        items = ", ".join(annotation_to_string(e) for e in elements)
        return f"({items})"

    if cls == "ExprList":
        elements = annotation.get("elements", [])
        items = ", ".join(annotation_to_string(e) for e in elements)
        return f"[{items}]"

    # Fallback: try canonical or name
    if "canonical" in annotation:
        return annotation["canonical"]
    if "name" in annotation:
        return annotation["name"]

    return ""


def process_parameter(param: dict[str, Any]) -> dict[str, Any]:
    """Process a function parameter.

    Parameters
    ----------
    param : dict
        Griffe parameter object.

    Returns
    -------
    dict
        Simplified parameter representation.
    """
    annotation = param.get("annotation")
    type_str = annotation_to_string(annotation)

    return {
        "name": param.get("name", ""),
        "type": type_str,
        "default": param.get("default"),
        "kind": param.get("kind", ""),
    }


def process_function(func_data: dict[str, Any]) -> dict[str, Any]:
    """Process a function definition.

    Parameters
    ----------
    func_data : dict
        Griffe function object.

    Returns
    -------
    dict
        Simplified function representation.
    """
    # Build signature string
    params = func_data.get("parameters", [])
    param_strs = []
    for p in params:
        if not isinstance(p, dict):
            continue
        pstr = p.get("name", "")
        type_str = annotation_to_string(p.get("annotation"))
        if type_str:
            pstr += f": {type_str}"
        if p.get("default") is not None:
            pstr += f" = {p['default']}"
        param_strs.append(pstr)

    signature = f"({', '.join(param_strs)})"

    # Get return type
    returns = func_data.get("returns")
    return_type = annotation_to_string(returns)

    return {
        "name": func_data.get("name", ""),
        "kind": "function",
        "signature": signature,
        "docstring": process_docstring(func_data.get("docstring")),
        "parameters": [process_parameter(p) for p in params if isinstance(p, dict)],
        "returns": {"type": return_type},
        "decorators": func_data.get("decorators", []),
    }


def process_class(cls_data: dict[str, Any]) -> dict[str, Any]:
    """Process a class definition.

    Parameters
    ----------
    cls_data : dict
        Griffe class object.

    Returns
    -------
    dict
        Simplified class representation.
    """
    members = cls_data.get("members", {})

    methods = {}
    attributes = {}

    for name, member in members.items():
        if not isinstance(member, dict):
            continue
        kind = member.get("kind", "")
        if kind == "function":
            methods[name] = process_function(member)
        elif kind == "attribute":
            type_str = annotation_to_string(member.get("annotation"))
            attributes[name] = {
                "name": name,
                "type": type_str,
                "docstring": process_docstring(member.get("docstring")),
                "value": member.get("value"),
            }

    bases = []
    for base in cls_data.get("bases", []):
        if isinstance(base, dict):
            bases.append(base.get("canonical", base.get("name", "")))
        elif isinstance(base, str):
            bases.append(base)

    return {
        "name": cls_data.get("name", ""),
        "kind": "class",
        "path": cls_data.get("path", ""),
        "docstring": process_docstring(cls_data.get("docstring")),
        "bases": bases,
        "methods": methods,
        "attributes": attributes,
    }


def process_module(mod_data: dict[str, Any]) -> dict[str, Any]:
    """Process a module definition.

    Parameters
    ----------
    mod_data : dict
        Griffe module object.

    Returns
    -------
    dict
        Simplified module representation.
    """
    members = mod_data.get("members", {})

    classes = {}
    functions = {}
    submodules = {}
    aliases = {}

    for name, member in members.items():
        if not isinstance(member, dict):
            continue
        kind = member.get("kind", "")
        if kind == "class":
            classes[name] = process_class(member)
        elif kind == "function":
            functions[name] = process_function(member)
        elif kind == "module":
            submodules[name] = process_module(member)
        elif kind == "alias":
            aliases[name] = {
                "name": name,
                "target": member.get("target_path", ""),
            }

    return {
        "name": mod_data.get("name", ""),
        "kind": "module",
        "path": mod_data.get("path", ""),
        "docstring": process_docstring(mod_data.get("docstring")),
        "classes": classes,
        "functions": functions,
        "modules": submodules,
        "aliases": aliases,
    }


def transform_for_nextjs(griffe_data: dict[str, Any]) -> dict[str, Any]:
    """Transform Griffe output to Next.js-friendly format.

    Parameters
    ----------
    griffe_data : dict
        Raw Griffe JSON output.

    Returns
    -------
    dict
        Transformed documentation suitable for Next.js consumption.
    """
    return process_module(griffe_data)


def build_navigation(packages: dict[str, Any]) -> list[dict[str, Any]]:
    """Build navigation structure from package docs.

    Parameters
    ----------
    packages : dict
        Package documentation keyed by package name.

    Returns
    -------
    list
        Navigation tree structure.
    """
    nav = []

    for pkg_name, pkg_data in packages.items():
        pkg_nav = {
            "title": pkg_name,
            "path": f"/docs/api/{pkg_name}",
            "children": [],
        }

        # Add modules as children
        for mod_name in sorted(pkg_data.get("modules", {}).keys()):
            pkg_nav["children"].append({
                "title": mod_name,
                "path": f"/docs/api/{pkg_name}/{mod_name}",
            })

        nav.append(pkg_nav)

    return nav


def main() -> int:
    """Generate documentation for all packages.

    Returns
    -------
    int
        Exit code (0 for success, 1 for failure).
    """
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    all_docs: dict[str, Any] = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "packages": {},
    }

    for pkg_name, pkg_path in PACKAGES:
        print(f"Generating docs for {pkg_name}...")
        try:
            raw_docs = generate_package_docs(pkg_name, pkg_path)
            # Griffe returns {pkg_name: {...}}
            pkg_docs = raw_docs.get(pkg_name, raw_docs)
            transformed = transform_for_nextjs(pkg_docs)
            all_docs["packages"][pkg_name] = transformed

            # Write individual package file
            pkg_file = OUTPUT_DIR / f"{pkg_name}.json"
            pkg_file.write_text(json.dumps(transformed, indent=2))
            print(f"  -> {pkg_file}")

        except Exception as e:
            print(f"  ERROR: {e}", file=sys.stderr)
            return 1

    # Build navigation
    all_docs["navigation"] = build_navigation(all_docs["packages"])

    # Write combined index
    index_file = OUTPUT_DIR / "index.json"
    index_file.write_text(json.dumps(all_docs, indent=2))
    print(f"  -> {index_file}")

    print(f"\nDocumentation generated in {OUTPUT_DIR}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
