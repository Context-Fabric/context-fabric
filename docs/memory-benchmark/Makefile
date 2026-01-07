TEX = 2025-01-memory-benchmark-tf-vs-cf
PDF = $(TEX).pdf

.PHONY: all clean

all: $(PDF)

$(PDF): $(TEX).tex
	pdflatex -interaction=nonstopmode $(TEX).tex
	pdflatex -interaction=nonstopmode $(TEX).tex

clean:
	rm -f *.aux *.log *.out *.toc *.lof *.lot *.bbl *.blg
