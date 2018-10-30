use pdf_canvas::Pdf;

use parser;
use layout::Layout;
use layout::BurroBox;

pub fn write_document(root: parser::Node, path: &str) {
    match root {
        parser::Node::Document(_) => {},
        _ => panic!("expected document, got other node"),
    }

    // 8.5 x 11 in points....
    let mut layout = Layout::new(612.0, 792.0);
    layout.construct(&root);

    let mut document = Pdf::create(path)
        .expect("Create pdf file");

    document.render_page(612.0, 792.0, |canvas| {
        for b in layout.boxes {
            match b {
                BurroBox::Char(cb) => {
                    let font = canvas.get_font(cb.font);
                    canvas.text(|t| {
                        t.set_font(&font, cb.height)?;
                        t.set_leading(18.0)?;
                        t.pos(cb.x, cb.y)?;
                        t.show(&cb.c)?;
                        Ok(())
                    })?;
                }
            }
        };

        Ok(())
    }).expect("Write page");

    document.finish().expect("saved document");
}
