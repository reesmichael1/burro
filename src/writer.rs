use pdf_canvas::{Pdf, BuiltinFont};

use parser;
use layout::{Layout, BurroBox, Style};

pub fn write_document(root: parser::Node, path: &str) {
    match root {
        parser::Node::Document(_) => {},
        _ => panic!("expected document, got other node"),
    }

    // 8.5 x 11 in points....
    let mut layout = Layout::new(612.0, 792.0);
    layout.construct(&root);

    let mut document = Pdf::create(path)
        .expect("could not create PDF file");

    document.render_page(612.0, 792.0, |canvas| {
        for b in layout.boxes {
            match b {
                BurroBox::Char(cb) => {
                    let mut font = canvas.get_font(BuiltinFont::Times_Roman);
                    let mut height = cb.height;
                    for style in &cb.styles {
                        match style {
                            Style::Font(fs) => {
                                font = canvas.get_font(fs.font);
                                height = fs.point_size;
                            },
                        }
                    }
                    canvas.text(|t| {
                        t.set_font(&font, height)?;
                        t.pos(cb.x, cb.y)?;
                        t.show(&cb.c)?;
                        Ok(())
                    })?;
                }
            }
        };

        Ok(())
    }).expect("could not write page");

    document.finish().expect("could not save document");
}
