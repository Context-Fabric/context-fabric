TEX := intro-to-cf__rewrite
PDF := $(TEX).pdf

.PHONY: all clean

all: $(PDF)

$(PDF): $(TEX).tex \
	fig_latency_distribution.pdf \
	fig_latency_percentiles.pdf \
	fig_memory_multicorpus.pdf \
	fig_scaling_progressive.pdf
	pdflatex -interaction=nonstopmode -halt-on-error $(TEX).tex
	pdflatex -interaction=nonstopmode -halt-on-error $(TEX).tex

clean:
	rm -f *.aux *.log *.out *.toc *.lof *.lot *.bbl *.blg
