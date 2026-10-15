//! The text of a page's own article, the way a person sees it (#301): for a
//! link whose page carries no schema.org `Recipe`, read the way a paste is.
//!
//! **Only the article, never the whole page.** A page's sidebar, menu,
//! comments and footer are text too, and read as a paste they would land as
//! Ingredient Lines. So the article is found first, the way a reader would:
//!
//! 1. The element microdata marks `itemprop="articleBody"`. Blogger and many
//!    WordPress themes say it in so many words, and a page that names its own
//!    article is believed.
//! 2. Otherwise the longest `<article>`.
//! 3. Otherwise the block holding the most prose, found the way Mozilla's
//!    Readability finds it: each paragraph of 25 letters or more counts its
//!    length toward the block it sits in, and half toward the one around
//!    that, and blocks whose class or id says comments, sidebar, menu and the
//!    like are passed over.
//!
//! **Then it is written out as lines, the way a browser copies it.** A block
//! starts a line, `<br>` ends one, a paragraph or heading stands between
//! blank lines, and runs of spaces inside a line are one space. That is
//! rendering the HTML, not rewriting the text (ADR 0002): what comes out is
//! what a person who selected the article and copied it would paste, so the
//! paste reader makes the same recipe of either (#301).

use std::collections::HashMap;

use scraper::{ElementRef, Html, Node, Selector};

/// How the article was found, which decides what inside it is read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Found {
    /// The page said which it is. It is taken whole.
    Marked,
    /// It was guessed at, so what its class or id says is not the post is
    /// passed over inside it too.
    Guessed,
}

/// The article's text, one line per line a browser shows, or nothing where
/// the page has no text to read.
///
/// An article that writes out as nothing, its only text inside a script,
/// is no article, and the next way of finding one is tried.
pub fn main_text(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    [marked_article(&document), longest_article(&document)]
        .into_iter()
        .flatten()
        .find_map(|root| written_out(root, Found::Marked))
        .or_else(|| written_out(most_prose(&document)?, Found::Guessed))
}

/// An article written out as lines, or nothing where it holds no text.
fn written_out(root: ElementRef<'_>, found: Found) -> Option<String> {
    let mut lines = Lines::default();
    lines.walk(root, found);
    lines.finish()
}

fn marked_article(document: &Html) -> Option<ElementRef<'_>> {
    let selector = Selector::parse(r#"[itemprop~="articleBody"]"#).ok()?;
    document.select(&selector).find(|found| has_text(*found))
}

fn longest_article(document: &Html) -> Option<ElementRef<'_>> {
    let selector = Selector::parse("article").ok()?;
    document
        .select(&selector)
        .filter(|found| has_text(*found))
        .max_by_key(|found| found.text().map(str::len).sum::<usize>())
}

fn has_text(element: ElementRef<'_>) -> bool {
    element.text().any(|text| !text.trim().is_empty())
}

/// Readability's shortest paragraph worth counting.
const SHORTEST_PARAGRAPH: usize = 25;

/// The blocks whose own text a paragraph is counted from.
fn holds_paragraphs(name: &str) -> bool {
    matches!(
        name,
        "p" | "pre" | "td" | "li" | "blockquote" | "dd" | "div"
    )
}

/// The block whose paragraphs hold the most prose (Readability's scoring,
/// cut down to what a recipe page needs).
///
/// One pass down the page, which leaves out the whole of a block passed
/// over, kept in a list of its own rather than by calling itself: a page
/// nested twenty thousand deep is a page anybody can link to, and it must
/// not run Kamosu out of stack.
fn most_prose(document: &Html) -> Option<ElementRef<'_>> {
    let mut scores = HashMap::new();
    let mut waiting = vec![document.root_element()];
    while let Some(element) = waiting.pop() {
        if outside_the_post(element, Found::Guessed) {
            continue;
        }
        if holds_paragraphs(element.value().name()) {
            let length = own_text_length(element);
            if length >= SHORTEST_PARAGRAPH {
                let parent = element.parent().and_then(ElementRef::wrap);
                let grandparent =
                    parent.and_then(|parent| parent.parent().and_then(ElementRef::wrap));
                for (block, amount) in [(parent, length * 2), (grandparent, length)] {
                    if let Some(block) = block {
                        *scores.entry(block.id()).or_insert(0usize) += amount;
                    }
                }
            }
        }
        waiting.extend(element.child_elements());
    }
    // A tie goes to the block that comes first on the page, so the same page
    // is read the same way every time it is imported.
    let best = scores.values().copied().max()?;
    document
        .root_element()
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|element| scores.get(&element.id()) == Some(&best))
}

/// How much text a block holds itself, not counting the blocks inside it.
fn own_text_length(block: ElementRef<'_>) -> usize {
    let mut length = 0;
    let mut waiting = vec![block];
    while let Some(element) = waiting.pop() {
        for child in element.children() {
            if let Some(text) = child.value().as_text() {
                length += text.trim().len();
            } else if let Some(inline) = ElementRef::wrap(child) {
                let name = inline.value().name();
                if !is_block(name) && !skipped(name) {
                    waiting.push(inline);
                }
            }
        }
    }
    length
}

/// An element that is no part of the post: never read text, and, in an
/// article that was guessed at, the parts of a page around a post.
///
/// The page itself is never around the post, whatever its class says: a
/// theme puts `has-sidebar` on `<body>` and means the page has one.
fn outside_the_post(element: ElementRef<'_>, found: Found) -> bool {
    let name = element.value().name();
    skipped(name)
        || (found == Found::Guessed
            && !matches!(name, "body" | "html")
            && (aside(name) || unlikely(element.value())))
}

/// What is never text a person reads.
fn skipped(name: &str) -> bool {
    matches!(
        name,
        "script"
            | "style"
            | "noscript"
            | "template"
            | "svg"
            | "iframe"
            | "form"
            | "button"
            | "select"
            | "textarea"
            | "canvas"
            | "head"
    )
}

/// The parts of a page that are around the post rather than in it.
fn aside(name: &str) -> bool {
    matches!(name, "nav" | "aside" | "footer" | "header")
}

/// Readability's list, cut down: a class or id saying the block is
/// comments, a sidebar, a menu or the like, unless it also says it is the
/// content.
fn unlikely(element: &scraper::node::Element) -> bool {
    const UNLIKELY: &[&str] = &[
        "comment",
        "sidebar",
        "footer",
        "header",
        "menu",
        "nav",
        "banner",
        "related",
        "share",
        "social",
        "sponsor",
        "widget",
        "popular",
        "breadcrumb",
        "pagination",
        "popup",
        "cookie",
        "newsletter",
        "disqus",
        "advert",
    ];
    const LIKELY: &[&str] = &["article", "content", "main", "post", "entry", "recipe"];
    let named = [element.attr("class"), element.attr("id")]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    UNLIKELY.iter().any(|word| named.contains(word))
        && !LIKELY.iter().any(|word| named.contains(word))
}

fn is_block(name: &str) -> bool {
    matches!(
        name,
        "address"
            | "article"
            | "aside"
            | "blockquote"
            | "dd"
            | "details"
            | "div"
            | "dl"
            | "dt"
            | "fieldset"
            | "figcaption"
            | "figure"
            | "footer"
            | "header"
            | "hr"
            | "li"
            | "main"
            | "nav"
            | "ol"
            | "section"
            | "summary"
            | "table"
            | "tbody"
            | "thead"
            | "tfoot"
            | "tr"
            | "td"
            | "th"
            | "ul"
            | "body"
            | "html"
            | "caption"
    ) || stands_apart(name)
}

/// Blocks a browser sets between blank lines.
fn stands_apart(name: &str) -> bool {
    matches!(name, "p" | "pre" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

/// The lines written so far, and the one being written.
#[derive(Default)]
struct Lines {
    done: Vec<String>,
    current: String,
    /// Whitespace was met since the last letter, and is one space if a
    /// letter follows on the same line.
    space: bool,
}

impl Lines {
    /// One step at a time from a list rather than by calling itself, for
    /// the reason [`most_prose`] gives.
    fn walk(&mut self, root: ElementRef<'_>, found: Found) {
        enum Step<'a> {
            Text(&'a str),
            Open(ElementRef<'a>),
            Close(&'a str),
        }
        fn children<'a>(element: ElementRef<'a>) -> Vec<Step<'a>> {
            let mut steps = Vec::new();
            for child in element.children() {
                match child.value() {
                    Node::Text(text) => steps.push(Step::Text(text)),
                    Node::Element(_) => steps.extend(ElementRef::wrap(child).map(Step::Open)),
                    _ => {}
                }
            }
            steps.reverse();
            steps
        }
        let mut waiting = children(root);
        while let Some(step) = waiting.pop() {
            match step {
                Step::Text(text) => self.write(text),
                Step::Open(element) => {
                    let name = element.value().name();
                    if outside_the_post(element, found) {
                        continue;
                    }
                    if name == "br" {
                        self.end_line();
                        continue;
                    }
                    self.edge(name);
                    waiting.push(Step::Close(name));
                    waiting.extend(children(element));
                }
                Step::Close(name) => self.edge(name),
            }
        }
    }

    /// Where an element starts or ends: a block's edge ends a line, and a
    /// paragraph or heading has a blank line on either side.
    fn edge(&mut self, name: &str) {
        if is_block(name) {
            self.end_block();
        }
        if stands_apart(name) {
            self.blank();
        }
    }

    fn write(&mut self, text: &str) {
        for letter in text.chars() {
            // A no-break space is a letter a browser does not fold away, and
            // it copies as itself.
            if letter.is_whitespace() && letter != '\u{a0}' {
                self.space = true;
                continue;
            }
            if self.space && !self.current.is_empty() {
                self.current.push(' ');
            }
            self.space = false;
            self.current.push(letter);
        }
    }

    /// `<br>`: the line ends here, empty or not.
    fn end_line(&mut self) {
        self.done.push(self.current.trim().to_string());
        self.current.clear();
        self.space = false;
    }

    /// A block's edge ends a line that has something on it.
    fn end_block(&mut self) {
        if !self.current.trim().is_empty() {
            self.end_line();
        } else {
            self.current.clear();
            self.space = false;
        }
    }

    /// One blank line, never two in a row of its own making.
    fn blank(&mut self) {
        if self.done.last().is_some_and(|last| !last.is_empty()) {
            self.done.push(String::new());
        }
    }

    fn finish(mut self) -> Option<String> {
        self.end_block();
        let text = self.done.join("\n");
        let text = text.trim_matches('\n');
        (!text.trim().is_empty()).then(|| text.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Blogger's shape: the post marked with microdata, a sidebar and
    /// comments around it. `itemprop` holds two names here, as it does on
    /// the desktop page.
    const BLOGGER: &str = r#"<html><head><title>Cari Bo | A Blog</title>
<script>var tracking = "Ingredients";</script></head><body>
<div class="sidebar"><h2>Popular posts</h2><p>Ten soups for winter evenings, all of them easy.</p></div>
<h3 class="post-title">Vietnamese Beef Curry</h3>
<div class="post-body" itemprop="description articleBody">
<div>&nbsp;<a href="/photo.jpg"><img src="/photo.jpg"></a></div><div><b>Cari Bo</b></div><div><br />
</div><div>Hi there!&nbsp; It has been <b>a whole</b> week.</div><div><br />
</div><div><br />
<b><span>Ingredients</span></b><br />
<br />
<span>900g / 2 lb stewing beef, cut into 5cm / 2          inch cubes</span></div><div><span>2 tbsp grated fresh ginger</span></div>
<div><span><b>Method :</b></span><br />
<span><b>&nbsp;</b> </span></div><span>Combine the
  beef and the ginger.</span><br /><br /><span>Simmer for an hour.</span>
</div>
<div class="comments"><p>Lovely recipe, thank you so much for sharing it with us!</p></div>
</body></html>"#;

    #[test]
    fn a_post_marked_as_the_article_body_is_read_as_a_person_sees_it() {
        assert_eq!(
            main_text(BLOGGER).as_deref(),
            Some(
                "Cari Bo\n\nHi there!\u{a0} It has been a whole week.\n\n\nIngredients\n\n\
                 900g / 2 lb stewing beef, cut into 5cm / 2 inch cubes\n\
                 2 tbsp grated fresh ginger\nMethod :\nCombine the beef and the ginger.\n\n\
                 Simmer for an hour."
            )
        );
    }

    #[test]
    fn an_article_element_is_the_post_where_no_microdata_says_so() {
        let html = "<html><body><nav>Home Recipes About</nav>\
            <article><h1>Lentil Soup</h1><p>A soup for a cold day.</p>\
            <ul><li>200 g red lentils</li><li>1 onion</li></ul></article>\
            <footer>All rights reserved.</footer></body></html>";
        assert_eq!(
            main_text(html).as_deref(),
            Some("Lentil Soup\n\nA soup for a cold day.\n\n200 g red lentils\n1 onion")
        );
    }

    #[test]
    fn otherwise_the_block_holding_the_most_prose_is_the_post() {
        let html = r#"<html><body>
            <div id="menu"><a href="/">Home</a><a href="/about">About this site and its author</a></div>
            <div class="wrap"><div class="content">
              <h2>Tomato Tart</h2>
              <p>This tart is the best thing to do with tomatoes at the end of the summer.</p>
              <ul><li>1 sheet of puff pastry</li><li>4 ripe tomatoes, sliced</li></ul>
              <p>Lay the tomatoes over the pastry and bake it until it is golden all over.</p>
            </div>
            <div id="comments"><p>I made this last night and the whole family loved it, thank you!</p></div>
            </div></body></html>"#;
        assert_eq!(
            main_text(html).as_deref(),
            Some(
                "Tomato Tart\n\n\
                 This tart is the best thing to do with tomatoes at the end of the summer.\n\n\
                 1 sheet of puff pastry\n4 ripe tomatoes, sliced\n\n\
                 Lay the tomatoes over the pastry and bake it until it is golden all over."
            )
        );
    }

    #[test]
    fn a_page_nested_far_deeper_than_any_real_one_is_read_on_a_small_stack() {
        // A Job runs on a thread with tokio's 2 MiB stack. Nothing marks the
        // article, so the page is scored and then written out, both all the
        // way down.
        let deep = format!(
            "<html><body>{}<p>Beat the eggs into the flour until it is smooth.</p>{}</body></html>",
            "<div>".repeat(20_000),
            "</div>".repeat(20_000),
        );
        let read = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(move || main_text(&deep))
            .unwrap()
            .join()
            .expect("no overflow");
        assert_eq!(
            read.as_deref(),
            Some("Beat the eggs into the flour until it is smooth.")
        );
    }

    #[test]
    fn a_page_whose_body_says_it_has_a_sidebar_is_still_read() {
        let html = r#"<html><body class="page-template-default has-sidebar"><div class="content">
            <p>This tart is the best thing to do with tomatoes at the end of the summer.</p>
            </div></body></html>"#;
        assert_eq!(
            main_text(html).as_deref(),
            Some("This tart is the best thing to do with tomatoes at the end of the summer.")
        );
    }

    #[test]
    fn an_article_with_no_text_but_a_script_is_passed_by() {
        let html = r#"<html><body><article><script>var x = "a long script string";</script></article>
            <div><p>This tart is the best thing to do with tomatoes at the end of the summer.</p></div>
            </body></html>"#;
        assert_eq!(
            main_text(html).as_deref(),
            Some("This tart is the best thing to do with tomatoes at the end of the summer.")
        );
    }

    #[test]
    fn blocks_that_score_the_same_are_settled_by_which_comes_first() {
        // Each block scores twice its paragraph and the page once each,
        // which ties all three. The page comes first, and holds both.
        let html = "<html><body>\
            <div id=a><p>Block one of the prose on this page, long enough.</p></div>\
            <div id=b><p>Block two of the prose on this page, long enough.</p></div>\
            </body></html>";
        for _ in 0..20 {
            assert_eq!(
                main_text(html).as_deref(),
                Some(
                    "Block one of the prose on this page, long enough.\n\n\
                     Block two of the prose on this page, long enough."
                )
            );
        }
    }

    #[test]
    fn a_page_with_no_text_has_no_article() {
        assert_eq!(main_text("<html><body><img src=x></body></html>"), None);
        assert_eq!(main_text(""), None);
    }
}
