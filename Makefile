LATEX = pdflatex
FLAGS = -interaction=nonstopmode -halt-on-error

all:
	cargo build --release --features cli

coverage:
	cargo tarpaulin

test:
	cargo test

docs:
	cargo doc --open
	$(LATEX) $(FLAGS) report.tex

ext1:
	trunk serve --no-default-features --features gui

clean:
	cargo clean
	trunk clean