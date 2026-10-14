# HIPHIL Novum submission build

This directory contains the anonymous Microsoft Word submission and the matching PDF copy. The build uses the journal's official Word template and the LaTeX article in the parent directory as its content source.

Build the editable submission:

```sh
../.venv/bin/pip install -r requirements.txt
../.venv/bin/python build_submission.py
```

The script downloads the official template on first use, renders the LaTeX visual assets at 300 dpi, removes author and review metadata, and applies the template's named `HIPHIL` styles. Microsoft Word is used separately to write the exact PDF copy.

Generated outputs:

- `intro-to-cf__hiphil-novum.docx`
- `intro-to-cf__hiphil-novum.pdf`
