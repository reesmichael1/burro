# Burro

A digital typesetting language that's easy to type.

## Vision

`burro` is an attempt to find a middle ground between the verbosity of [`TeX`](http://tug.org), the excessive newlines of [`groff`](https://www.gnu.org/software/groff/), and the limited flexibility of compiling to a PDF from Markdown. A fundamental guiding principle is that the language _should be easy to type_, since it is, well, a typesetting language. `burro` will also have strong cross-reference support.

`burro` follows a simple syntax, where a dot `.` begins a command, which then takes an argument. The argument is ended by the pipe `|`, or optionally by a double line break (which also starts a new paragraph).

`burro` is meant for easy typesetting of mostly-text documents. Its closest spiritual ancestor is [`groff`](https://www.gnu.org/software/groff/), particularly with the [`mom`](http://www.schaffter.ca/mom/) macro set. 

## Example

The following is valid syntax:

```
.title Hello World

This .bold is a new| typesetting .italic language|. 
```

(However, do note that this example will not compile, because the `.title` command has not yet been implemented.)

Saved as `example.bur`, this file can be compiled with `burro example.bur`. The command will output `example.pdf`. 

## Current Status 

Currently, `burro` can produce simple single-page PDFs. The file `examples/demo.bur` is maintained as a demonstration of `burro`'s feature set.

## Roadmap

An incomplete but still useful list of coming features sorted by planned order of implementation:

- Support for multiple pages
- Other text justfication modes
- Allow for user configuration of typesetting parameters (margins, page size, leading, etc.)
- Useful commands for common document features (`.title`, `.author`, etc.)
- Allow user to specify output file
- Provide commands for cross references and bibliographies
