# Burro

A digital typesetting language that's easy to type.

## Vision

Burro is an attempt to find a middle ground between the verbosity of [`TeX`](http://tug.org), the excessive newlines of [`groff`](https://www.gnu.org/software/groff/), and the limited flexibility and power of compiling to a PDF from Markdown. A fundamental guiding principle is that the language _should be easy to type_, since it is, well, a typesetting language. 

Burro is meant for easy typesetting of mostly-text documents (i.e., it makes no attempt to replace TeX and friends for scientific documents). It is directly inspired by, and aims to be a spiritual successor to, [`groff`](https://www.gnu.org/software/groff/) with the [`mom`](http://www.schaffter.ca/mom/) macro set. 

It is also important that the user should have easy yet complete control over where everything goes on the page. Burro aims to have sensible defaults (mostly taken from Bringhurst's classic _The Elements of Typographic Style_) that can all be customized, anywhere in the document.

## Example

```
.start

.align[center]
.bold[Burro]

.align[justify]
Hello world! This is an example .italic[Burro] document.

.pt_size[18]
Now, this line has a larger point size than the preceding one.
```

For now, Burro requires a font map, telling it where to find font files for each family/font combination it encounters. See `examples/fontmap` for the necessary syntax.

Saved as `example.bur`, this file can be compiled with `burro example.bur` (assuming the fontmap is stored next to `example.bur`). The command will output `example.pdf`. 

## Project History

I first worked on Burro in 2018, but couldn't settle on a syntax I liked, so the project sat dormant for some time. This latest rewrite is already more powerful than the original version was, and has a much more flexible and pleasant syntax.

## Roadmap

These are many more features planned for Buro, including an extensive list of typographic commands, user defined variables, and more. See the file `examples/syntax.bur` for the current long term vision. Burro is not yet tested on non-Latin scripts (or even non-English languages), but support is a long-term goal.

Burro is not yet documented, but we will have good documentation of all commands and configuration before the first major release.
