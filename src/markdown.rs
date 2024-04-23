use markdown::{mdast::Node, ParseOptions};
use roff::{bold, italic, line_break, roman, Inline};

enum List {
    Simple,
    Numbered(u32),
}

impl List {
    fn bullet(&mut self) -> String {
        match self {
            List::Simple => "-".to_string(),
            List::Numbered(i) => {
                let x = i.to_string();
                *i += 1;
                x
            }
        }
    }
}

#[derive(Default)]
struct State {
    bold: bool,
    italic: bool,
    start: bool,
    indentation: usize,
    list_bullets: Vec<List>,
}

impl State {
    fn fmt(&self, s: impl ToString) -> Inline {
        let s = s.to_string();
        if self.bold {
            bold(s)
        } else if self.italic {
            italic(s)
        } else {
            roman(s)
        }
    }
}

fn traverse_nodes(node: &Node, inline: &mut Vec<Inline>, state: &mut State) {
    match node {
        Node::BlockQuote(_) => state.indentation += 2,
        Node::List(list) => {
            state.indentation += 1;
            let bullet = list.start.map(List::Numbered).unwrap_or(List::Simple);
            state.list_bullets.push(bullet);
        }
        Node::InlineCode(code) => inline.push(state.fmt(format!("`{}`", code.value))),
        Node::Delete(_) => inline.push(state.fmt("~~")),
        Node::Emphasis(_) => state.italic = true,
        // TODO: improve link handling
        // Node::Link(_) => inline.push(state.fmt("  ".repeat(state.indentation) + "[")),
        Node::Strong(_) => state.bold = true,
        Node::Heading(h) => {
            inline.push(line_break());
            inline.push(state.fmt(" "));
            inline.push(state.fmt("  ".repeat(state.indentation)));
            inline.push(state.fmt("#".repeat(h.depth as usize)));
            inline.push(state.fmt(" "));
            state.bold = true;
        }
        Node::Text(s) => inline.push(state.fmt(&s.value)),
        Node::Code(code) => {
            inline.push(line_break());
            inline.push(line_break());

            for line in code.value.lines() {
                if line.starts_with("# ") {
                    continue;
                }
                let indent = "  ".repeat(state.indentation + 1);
                inline.push(state.fmt(indent));
                inline.push(state.fmt(line));
                inline.push(line_break());
            }

            // inline.push(line_break());
            inline.push(line_break());
        }
        Node::Paragraph(_) => {
            if state.start {
                let indent = "  ".repeat(state.indentation);
                inline.push(state.fmt(indent));
            }

            state.start = false;
        }
        Node::ListItem(_) => {
            let indent = "  ".repeat(state.indentation);
            inline.push(state.fmt(indent));

            let bullet = state.list_bullets.last_mut().unwrap().bullet();
            inline.push(state.fmt(bullet));
            inline.push(state.fmt(" "));

            state.start = false;
        }
        Node::ThematicBreak(_) => inline.push(line_break()),
        _ => {}
    }

    for child in node.children().into_iter().flatten() {
        traverse_nodes(child, inline, state);
    }

    match node {
        Node::BlockQuote(_) => state.indentation -= 2,
        Node::List(_) => {
            state.indentation -= 1;
            state.list_bullets.pop();
            state.start = true;
        }
        Node::ListItem(_) => state.start = true,
        Node::Delete(_) => inline.push(state.fmt("~~")),
        Node::Emphasis(_) => state.italic = false,
        // Node::Link(link) => inline.push(state.fmt(format!("]({})", link.url))),
        Node::Strong(_) => state.bold = false,
        Node::Heading(_) => {
            state.bold = false;
            inline.push(line_break());
            state.start = true;
        }
        Node::Paragraph(_) | Node::ThematicBreak(_) => {
            inline.push(line_break());
            state.start = true;
        }
        _ => state.start = true,
    }
}

pub fn to_roff(markdown: &str, indentation: usize) -> Vec<Inline> {
    let mut inline = Vec::new();
    let root = markdown::to_mdast(markdown, &ParseOptions::default()).unwrap();
    traverse_nodes(
        &root,
        &mut inline,
        &mut State {
            indentation,
            ..State::default()
        },
    );
    inline
}
