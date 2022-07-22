use std::cell::Cell;

use super::TagHandler;
use super::StructuredPrinter;

use markup5ever_rcdom::Handle;
use markup5ever_rcdom::NodeData;

thread_local! {
    static LAST_OL_START_INDEX: Cell<Option<u32>> = Cell::new(None);
}

#[derive(Default)]
pub(super) struct ListHandler {
    saved_last_ol_start_index: Option<u32>,
}

impl TagHandler for ListHandler {

    /// we're entering "ul" or "ol" tag, no "li" handling here
    fn handle(&mut self, tag: &Handle, printer: &mut StructuredPrinter) {
        LAST_OL_START_INDEX.with(|x| {
            self.saved_last_ol_start_index = x.get();
            match &tag.data {
                NodeData::Element { name, attrs, .. } if name.local == *"ol" => {
                    let attrs = attrs.borrow();
                    let value = attrs.iter().find(|x| x.name.local == *"start").and_then(|x| x.value.parse::<u32>().ok());
                    x.set(value);
                }
                _ => {
                    x.set(None);
                }
            }
        });
        printer.insert_newline();
    }

    /// indent now-ready list
    fn after_handle(&mut self, printer: &mut StructuredPrinter) {
        LAST_OL_START_INDEX.with(|x| x.set(self.saved_last_ol_start_index));
        printer.insert_newline();
        printer.insert_newline();
    }
}

#[derive(Default)]
pub struct ListItemHandler {
    start_pos: usize,
    list_type: String
}

impl TagHandler for ListItemHandler {

    fn handle(&mut self, _tag: &Handle, printer: &mut StructuredPrinter) {
        {
            let parent_lists: Vec<&String> = printer.parent_chain.iter().rev().filter(|&tag| tag == "ul" || tag == "ol" || tag == "menu").collect();
            let nearest_parent_list = parent_lists.first();
            if nearest_parent_list.is_none() {
                // no parent list
                // should not happen - html5ever cleans html input when parsing
                return;
            }

            self.list_type = nearest_parent_list.unwrap().to_string();
        }

        if printer.data.chars().last() != Some('\n') {
            // insert newline when declaring a list item only in case there isn't any newline at the end of text
            printer.insert_newline(); 
        }

        let current_depth = printer.parent_chain.len();
        let start = LAST_OL_START_INDEX.with(|x| x.get()).unwrap_or(1) as usize;
        let order = printer.siblings[&current_depth].len() + start;
        match self.list_type.as_ref() {
            "ul" | "menu" => printer.append_str("* "), // unordered list: *, *, *
            "ol" => printer.append_str(&(order.to_string() + ". ")), // ordered list: 1, 2, 3
            _ => {} // never happens
        }

        self.start_pos = printer.data.len();
    }

    fn after_handle(&mut self, printer: &mut StructuredPrinter) {
        let padding = match self.list_type.as_ref() {
            "ul" => 2,
            "ol" => 3,
            _ => 4
        };

        // need to cleanup leading newlines, <p> inside <li> should produce valid 
        // list element, not an empty line
        let index = self.start_pos;
        while index < printer.data.len() {
            if printer.data.bytes().nth(index) == Some(b'\n') || printer.data.bytes().nth(index) == Some(b' ') {
                printer.data.remove(index);
            } else {
                break;
            }
        }

        // non-nested indentation (padding). Markdown requires that all paragraphs in the
        // list item except first should be indented with at least 1 space
        let mut index = printer.data.len();
        while index > self.start_pos {
            if printer.data.bytes().nth(index) == Some(b'\n') {
                printer.insert_str(index + 1, &" ".repeat(padding));
            }
            index -= 1;
        }
    }
}