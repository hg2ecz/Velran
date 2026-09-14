TEX=main.tex
PDF=velran-for-web-application-developers.pdf
LATEXMK=latexmk

.PHONY: all pdf hu books clean distclean

all: pdf

pdf:
	$(LATEXMK) -pdf -interaction=nonstopmode -halt-on-error -jobname=velran-for-web-application-developers $(TEX)

hu:
	$(MAKE) -C hu pdf

books: pdf hu

clean:
	$(LATEXMK) -c -jobname=velran-for-web-application-developers $(TEX)
	$(MAKE) -C hu clean

# PDFs are build artifacts too; distclean leaves editable sources only.
distclean:
	$(LATEXMK) -C -jobname=velran-for-web-application-developers $(TEX)
	rm -f $(PDF)
	$(MAKE) -C hu distclean
