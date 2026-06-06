#!/usr/bin/env python3
"""
Download and organize Text-Fabric corpora for benchmarking.

Downloads 10 biblical studies corpora, keeping only .tf files.
Each corpus is organized as: .corpora/{corpus}/tf/*.tf with a README.md

Usage:
    python benchmarks/download_corpora.py
"""

import os
import shutil
import subprocess
import tempfile
from datetime import datetime
from pathlib import Path

# Configuration for each corpus
CORPORA: dict[str, dict[str, str]] = {
    "bhsa": {
        "repo": "ETCBC/bhsa",
        "tf_path": "tf/2021",
        "description": "Biblia Hebraica Stuttgartensia Amstelodamensis (2021)",
        "language": "Hebrew",
    },
    "lxx": {
        "repo": "CenterBLC/LXX",
        "tf_path": "tf/1935",
        "description": "Septuagint (Rahlfs' LXX Edition 1935)",
        "language": "Greek",
    },
    "n1904": {
        "repo": "CenterBLC/N1904",
        "tf_path": "tf/1.0.0",
        "description": "Nestle 1904 Greek New Testament",
        "language": "Greek",
    },
    "banks": {
        "repo": "annotation/banks",
        "tf_path": "tf/0.2",
        "description": "Banks example corpus with explicit structure hierarchy",
        "language": "English",
    },
    "dss": {
        "repo": "ETCBC/dss",
        "tf_path": "tf/1.9",
        "description": "Dead Sea Scrolls (Martin Abegg data)",
        "language": "Hebrew",
    },
    "peshitta": {
        "repo": "ETCBC/peshitta",
        "tf_path": "tf/0.2",
        "description": "Syriac Old Testament (Peshitta)",
        "language": "Syriac",
    },
    "syrnt": {
        "repo": "ETCBC/syrnt",
        "tf_path": "tf/0.1",
        "description": "Syriac New Testament",
        "language": "Syriac",
    },
    "sp": {
        "repo": "DT-UCPH/sp",
        "tf_path": "tf/4.1.2",
        "description": "Samaritan Pentateuch",
        "language": "Hebrew",
    },
    "tischendorf": {
        "repo": "codykingham/tischendorf_tf",
        "tf_path": "tf/2.8",
        "description": "Tischendorf 8th Edition Greek NT",
        "language": "Greek",
    },
    "quran": {
        "repo": "q-ran/quran",
        "tf_path": "tf/0.3",
        "description": "Quranic Arabic Corpus",
        "language": "Arabic",
    },
    "cuc": {
        "repo": "DT-UCPH/cuc",
        "tf_path": "tf/0.1.4",
        "description": "Copenhagen Ugaritic Corpus",
        "language": "Ugaritic",
    },
}


def copy_tf_files(src: Path, dst: Path) -> int:
    """Copy only .tf files from src to dst, excluding cache dirs."""
    dst.mkdir(parents=True, exist_ok=True)
    count = 0
    for tf_file in src.glob("*.tf"):
        if tf_file.is_file():
            shutil.copy2(tf_file, dst / tf_file.name)
            count += 1
    return count


def generate_readme(corpus_name: str, config: dict, tf_dir: Path) -> str:
    """Generate README.md content for a corpus."""
    tf_count = len(list(tf_dir.glob("*.tf")))

    if "repo" in config:
        source_line = f"**Repository:** https://github.com/{config['repo']}"
    else:
        source_line = f"**Source:** {config.get('url', config.get('source', 'Unknown'))}"

    return f"""# {corpus_name.upper()}

{config['description']}

## Source

{source_line}

## Details

- **Language:** {config['language']}
- **TF Files:** {tf_count}
- **Downloaded:** {datetime.now().strftime('%Y-%m-%d')}

## Usage

```python
from tf.fabric import Fabric
TF = Fabric(locations='{tf_dir}')
api = TF.loadAll()
```
"""


def download_from_github(repo: str, tf_path: str, dest: Path) -> int:
    """Clone a GitHub repo and copy TF files."""
    with tempfile.TemporaryDirectory() as tmpdir:
        clone_path = Path(tmpdir) / "repo"

        print(f"  Cloning https://github.com/{repo}...")
        result = subprocess.run(
            ["git", "clone", "--depth", "1", f"https://github.com/{repo}.git", str(clone_path)],
            capture_output=True,
            text=True,
        )

        if result.returncode != 0:
            print(f"  ERROR: Failed to clone {repo}")
            print(f"  {result.stderr}")
            return 0

        # Find the TF directory
        src_tf = clone_path / tf_path
        if not src_tf.exists():
            print(f"  ERROR: TF path not found: {tf_path}")
            return 0

        # Copy TF files
        count = copy_tf_files(src_tf, dest)
        print(f"  Copied {count} .tf files")
        return count


def copy_from_local(source: str, dest: Path) -> int:
    """Copy TF files from a local directory."""
    src_path = Path(source).expanduser()

    if not src_path.exists():
        print(f"  ERROR: Local path not found: {src_path}")
        return 0

    # Copy only .tf files, excluding cache directories
    count = copy_tf_files(src_path, dest)
    print(f"  Copied {count} .tf files from {src_path}")
    return count


def main():
    """Download all corpora."""
    # Output to package root (.corpora at libs/benchmarks/ level)
    script_dir = Path(__file__).parent
    corpora_dir = script_dir.parent.parent / ".corpora"

    print("=" * 60)
    print("Biblical Studies Corpora Downloader")
    print("=" * 60)
    print(f"\nOutput directory: {corpora_dir}")

    success_count = 0

    for corpus_name, config in CORPORA.items():
        print(f"\n[{corpus_name}] {config['description']}")

        corpus_dir = corpora_dir / corpus_name
        tf_dir = corpus_dir / "tf"

        # Remove existing corpus directory for fresh download
        if corpus_dir.exists():
            shutil.rmtree(corpus_dir)

        tf_dir.mkdir(parents=True, exist_ok=True)

        # Download or copy
        if "source" in config:
            count = copy_from_local(config["source"], tf_dir)
        else:
            count = download_from_github(config["repo"], config["tf_path"], tf_dir)

        if count > 0:
            # Generate README
            readme_content = generate_readme(corpus_name, config, tf_dir)
            readme_path = corpus_dir / "README.md"
            readme_path.write_text(readme_content)
            print(f"  Created README.md")
            success_count += 1
        else:
            print(f"  SKIPPED: No TF files found")

    print("\n" + "=" * 60)
    print(f"Download complete: {success_count}/{len(CORPORA)} corpora")
    print("=" * 60)


if __name__ == "__main__":
    main()
