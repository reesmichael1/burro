use std::collections::HashMap;

use printpdf::*;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use crate::fontmap::FontMap;
use crate::layout::{BurroBox, Layout, Page};

pub fn write_pdf(layout: &Layout, font_map: &FontMap, dest: &Path) {
    if layout.pages.len() == 0 {
        return;
    }

    let page = &layout.pages[0];
    let (page_width, page_height) = page_dimensions(page);
    // Once we have a document title command, we can set it here
    let (doc, page1, layer1) = PdfDocument::new("", page_width, page_height, "Layer 1");

    let mut fonts: HashMap<u32, IndirectFontRef> = HashMap::new();

    let mut current_layer = doc.get_page(page1).get_layer(layer1);

    for (ix, page) in layout.pages.iter().enumerate() {
        for bbox in &page.boxes[..] {
            match bbox {
                BurroBox::Glyph {
                    id,
                    pos,
                    font: font_id,
                    pts,
                } => {
                    current_layer.begin_text_section();
                    current_layer.set_text_cursor(Pt(pos.x).into(), Pt(pos.y).into());
                    if fonts.contains_key(font_id) {
                        let font = &fonts[font_id];
                        current_layer.set_font(font, *pts);
                    } else {
                        let path = font_map.font_from_id(*font_id).as_ref().unwrap();
                        let font = doc.add_external_font(File::open(&path).unwrap()).unwrap();
                        fonts.insert(*font_id, font.clone());
                        current_layer.set_font(&font, *pts);
                    }

                    current_layer.write_codepoints([*id as u16]);

                    current_layer.end_text_section();
                }
            }
        }

        if ix != layout.pages.len() - 1 {
            let next_page = &layout.pages[ix + 1];
            let (page_width, page_height) = page_dimensions(next_page);
            let (next_page, next_layer) = doc.add_page(page_width, page_height, "Layer 1");
            current_layer = doc.get_page(next_page).get_layer(next_layer);
        }
    }

    doc.save(&mut BufWriter::new(File::create(dest).unwrap()))
        .unwrap();
}

fn page_dimensions(page: &Page) -> (Mm, Mm) {
    (Pt(page.width).into(), Pt(page.height).into())
}
