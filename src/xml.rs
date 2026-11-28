// SPDX-License-Identifier: MIT OR Apache-2.0

//! The XML reader: the part of XML that XMP uses, read into a tree of
//! elements, attributes and text, with every name resolved to its
//! namespace.
//!
//! It is written from XML 1.0 (Fifth Edition) and Namespaces in XML 1.0
//! (Third Edition), for the XML of XMP packets and sidecars (ADR024). XMP
//! Part 3 asks for UTF-8 wherever a packet lies in a TIFF or JPEG file,
//! and every measured sidecar is UTF-8 too, so the reader reads UTF-8
//! alone. It reads:
//!
//! - an XML declaration at the start, which is checked: one that names
//!   another encoding than UTF-8, or version 1.1, is damage;
//! - elements, attributes and text, and names by XML's rules for the
//!   characters a name may hold;
//! - the five predefined entities, `&lt;`, `&gt;`, `&amp;`, `&apos;` and
//!   `&quot;`, and character references in decimal and in hexadecimal;
//! - CDATA sections, whose text is kept as written;
//! - namespaces, each name resolved to its namespace URI, with default
//!   namespaces and prefixes declared again in a child.
//!
//! Line ends are normalized as XML 1.0 asks (§2.11): CR LF and a lone CR
//! become LF. Each attribute value is normalized too (§3.3.3): a tab, a
//! line end or a space becomes one space. A character reference stays
//! the character it names.
//!
//! Comments and processing instructions are skipped, the packet wrapper,
//! `<?xpacket …?>`, among them. A leading byte order mark is skipped. So
//! are NUL bytes after the end, which a value in a file may hold past the
//! packet's text: they are no part of the XML. No packet of the public
//! files measured has them.
//!
//! Anything else is damage, which costs the whole packet: XML gives a
//! reader no way to go on past it. It is an [`ErrorKind::Malformed`]
//! error, whose message names what was found, at the byte where it
//! starts. The damage includes:
//!
//! - bytes that are not UTF-8, or that are UTF-16 or UTF-32;
//! - a DOCTYPE, or an entity that is not one of the five;
//! - a character XML does not allow, in the text or through a reference;
//! - a prefix no element declares, a name with a misplaced colon, or a
//!   namespace declaration that Namespaces in XML forbids;
//! - an attribute repeated in one tag, after prefixes are resolved, or
//!   `<` in an attribute value;
//! - an end tag that does not match its start tag, an element not
//!   closed, or text or a second element outside the root.
//!
//! The reader is one loop, never recursive, and each element it opens is
//! one level deeper: past [`Limits::max_xml_depth`] levels, it stops with
//! [`ErrorKind::LimitExceeded`].
//!
//! [`ErrorKind::Malformed`]: crate::ErrorKind::Malformed
//! [`ErrorKind::LimitExceeded`]: crate::ErrorKind::LimitExceeded

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

use crate::error::{Error, Result};
use crate::limits::Limits;
use crate::namespaces;

/// No node: no parent, no next sibling, no first child or no namespace.
const NONE: usize = usize::MAX;

/// The namespace of namespace declarations, which no prefix may be bound
/// to (Namespaces in XML 1.0, §3).
const XMLNS: &str = "http://www.w3.org/2000/xmlns/";

/// A document read by [`parse`]: its elements and text, each name with its
/// namespace resolved. Names, and text that needed no change, borrow from
/// the bytes it was read from.
pub(crate) struct Document<'a> {
    /// Every element and text node, in the order they start in the
    /// document; the root element first.
    nodes: Vec<Node<'a>>,
    /// The attributes of every element, each element's in one run.
    attributes: Vec<StoredAttribute<'a>>,
    /// Each namespace URI the document declares, once.
    namespaces: Vec<Cow<'a, str>>,
}

/// One node of a [`Document`], linked to its parent and its next sibling.
struct Node<'a> {
    parent: usize,
    next: usize,
    kind: Kind<'a>,
}

enum Kind<'a> {
    Element {
        /// Where its start tag begins, for error messages.
        offset: usize,
        namespace: usize,
        local: &'a str,
        attributes: Range<usize>,
        first: usize,
    },
    /// The text between two pieces of markup, references replaced and
    /// line ends normalized; comments and processing instructions inside
    /// it are left out.
    Text(Cow<'a, str>),
}

struct StoredAttribute<'a> {
    namespace: usize,
    local: &'a str,
    value: Cow<'a, str>,
}

impl<'a> Document<'a> {
    /// The root element.
    pub(crate) fn root(&self) -> Element<'_> {
        Element {
            document: self,
            index: 0,
        }
    }

    /// Every element, in the order its start tag comes in the document.
    pub(crate) fn elements(&self) -> impl Iterator<Item = Element<'_>> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node.kind, Kind::Element { .. }))
            .map(|(index, _)| Element {
                document: self,
                index,
            })
    }

    fn namespace(&self, id: usize) -> Option<&str> {
        self.namespaces.get(id).map(|uri| &**uri)
    }
}

/// An element of a [`Document`].
#[derive(Clone, Copy)]
pub(crate) struct Element<'d> {
    document: &'d Document<'d>,
    index: usize,
}

/// One attribute of an [`Element`]: its namespace, `None` for a name with
/// no prefix, its local name and its normalized value.
#[derive(Clone, Copy)]
pub(crate) struct Attribute<'d> {
    pub(crate) namespace: Option<&'d str>,
    pub(crate) local: &'d str,
    pub(crate) value: &'d str,
}

/// One node in an element's content: an element, or a run of text.
#[derive(Clone, Copy)]
pub(crate) enum Child<'d> {
    Element(Element<'d>),
    Text(&'d str),
}

impl<'d> Element<'d> {
    fn parts(self) -> (usize, usize, &'d str, Range<usize>, usize) {
        match self.document.nodes.get(self.index).map(|node| &node.kind) {
            Some(&Kind::Element {
                offset,
                namespace,
                local,
                ref attributes,
                first,
            }) => (offset, namespace, local, attributes.clone(), first),
            _ => (0, NONE, "", 0..0, NONE),
        }
    }

    /// Where the element's start tag begins, in bytes from the start.
    pub(crate) fn offset(self) -> usize {
        self.parts().0
    }

    /// The URI of the element's namespace; `None` when it has none.
    pub(crate) fn namespace(self) -> Option<&'d str> {
        self.document.namespace(self.parts().1)
    }

    /// The element's local name, its name without its prefix.
    pub(crate) fn local(self) -> &'d str {
        self.parts().2
    }

    /// Whether the element is `local` of the namespace `namespace`.
    pub(crate) fn is(self, namespace: &str, local: &str) -> bool {
        self.namespace() == Some(namespace) && self.local() == local
    }

    /// The element that holds this one; `None` for the root.
    pub(crate) fn parent(self) -> Option<Element<'d>> {
        let parent = self.document.nodes.get(self.index)?.parent;
        (parent != NONE).then_some(Element {
            document: self.document,
            index: parent,
        })
    }

    /// The element's attributes, in the order of its start tag, less the
    /// namespace declarations.
    pub(crate) fn attributes(self) -> impl Iterator<Item = Attribute<'d>> {
        let document = self.document;
        document
            .attributes
            .get(self.parts().3)
            .unwrap_or_default()
            .iter()
            .map(move |attribute| Attribute {
                namespace: document.namespace(attribute.namespace),
                local: attribute.local,
                value: &attribute.value,
            })
    }

    /// The value of the attribute `local` of the namespace `namespace`,
    /// `None` for one with no prefix.
    pub(crate) fn attribute(self, namespace: Option<&str>, local: &str) -> Option<&'d str> {
        self.attributes()
            .find(|attribute| attribute.namespace == namespace && attribute.local == local)
            .map(|attribute| attribute.value)
    }

    /// The element's content: its child elements and the runs of text
    /// between them, in order.
    pub(crate) fn children(self) -> Children<'d> {
        Children {
            document: self.document,
            next: self.parts().4,
        }
    }
}

/// The content of an [`Element`], as [`Element::children`] gives it.
pub(crate) struct Children<'d> {
    document: &'d Document<'d>,
    next: usize,
}

impl<'d> Iterator for Children<'d> {
    type Item = Child<'d>;

    fn next(&mut self) -> Option<Child<'d>> {
        let index = self.next;
        let node = self.document.nodes.get(index)?;
        self.next = node.next;
        Some(match &node.kind {
            Kind::Element { .. } => Child::Element(Element {
                document: self.document,
                index,
            }),
            Kind::Text(text) => Child::Text(text),
        })
    }
}

/// Whether `text` is all white space as XML counts it: spaces, tabs and
/// line ends.
pub(crate) fn is_blank(text: &str) -> bool {
    text.chars().all(is_space)
}

/// Reads `bytes` as an XML document, under `limits`'s cap on depth.
///
/// # Errors
///
/// [`Malformed`](crate::ErrorKind::Malformed) when `bytes` are not
/// well-formed XML that this reader reads, as the module docs list;
/// [`LimitExceeded`](crate::ErrorKind::LimitExceeded) when elements nest
/// deeper than [`Limits::max_xml_depth`].
pub(crate) fn parse<'a>(bytes: &'a [u8], limits: &Limits) -> Result<Document<'a>> {
    // NULs past the packet's text, which a value in a file may hold, are
    // no part of the XML.
    let end = bytes
        .iter()
        .rposition(|&byte| byte != 0)
        .map_or(0, |last| last + 1);
    let bytes = bytes.get(..end).unwrap_or_default();
    if is_utf16_or_utf32(bytes) {
        return Err(Error::malformed(
            "the XML is in UTF-16 or UTF-32, not UTF-8",
            0,
        ));
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|err| Error::malformed("the XML is not UTF-8", at(err.valid_up_to())))?;
    if let Some((position, _)) = text.char_indices().find(|&(_, c)| !is_char(c)) {
        return Err(Error::malformed(
            "the XML holds a character it does not allow",
            at(position),
        ));
    }
    let mut parser = Parser::new(text, limits.max_xml_depth());
    if text.starts_with('\u{FEFF}') {
        parser.pos = '\u{FEFF}'.len_utf8();
    }
    parser.document()?;
    Ok(parser.document)
}

/// Whether `bytes` start as UTF-16 or UTF-32 text does: with its byte
/// order mark, or with a NUL in the first two bytes, as `<` takes in
/// either.
fn is_utf16_or_utf32(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFE, 0xFF])
        || bytes.starts_with(&[0xFF, 0xFE])
        || bytes.first() == Some(&0)
        || bytes.get(1) == Some(&0)
}

/// A position in the text as an error's byte offset.
fn at(position: usize) -> u64 {
    u64::try_from(position).unwrap_or(u64::MAX)
}

/// An element whose end tag has not been read yet.
struct Open<'a> {
    node: usize,
    /// Its name as written, which its end tag must repeat.
    name: &'a str,
    /// Where its start tag begins.
    offset: usize,
    /// How many prefixes were declared before it: its own come after.
    declared: usize,
    /// Its last child so far, to link the next one after.
    last: usize,
}

/// The text read since the last child element or the start tag, kept as a
/// slice of the document while it is one, and copied once it is not.
enum Run {
    Empty,
    Slice(Range<usize>),
    Owned(String),
}

/// An attribute as its tag writes it, before its prefix is resolved.
struct RawAttribute<'a> {
    name: &'a str,
    value: Cow<'a, str>,
    offset: usize,
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    max_depth: u32,
    document: Document<'a>,
    /// The id of each namespace URI in `document.namespaces`.
    ids: HashMap<Cow<'a, str>, usize>,
    /// For each prefix, the namespaces bound to it by the open elements,
    /// innermost last; `NONE` for a default namespace undeclared.
    bindings: HashMap<&'a str, Vec<usize>>,
    /// Each prefix the open elements declared, in order.
    declared: Vec<&'a str>,
    open: Vec<Open<'a>>,
    run: Run,
    xml: usize,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str, max_depth: u32) -> Self {
        let mut parser = Self {
            text,
            pos: 0,
            max_depth,
            document: Document {
                nodes: Vec::new(),
                attributes: Vec::new(),
                namespaces: Vec::new(),
            },
            ids: HashMap::new(),
            bindings: HashMap::new(),
            declared: Vec::new(),
            open: Vec::new(),
            run: Run::Empty,
            xml: NONE,
        };
        // The prefix `xml` is bound to the XML namespace without being
        // declared (Namespaces in XML 1.0, §3).
        parser.xml = parser.intern(Cow::Borrowed(namespaces::XML));
        parser
    }

    fn rest(&self) -> &'a str {
        self.text.get(self.pos..).unwrap_or_default()
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.rest().starts_with(c);
        if found {
            self.pos += c.len_utf8();
        }
        found
    }

    fn eat_str(&mut self, s: &str) -> bool {
        let found = self.rest().starts_with(s);
        if found {
            self.pos += s.len();
        }
        found
    }

    /// Skips white space; whether there was any.
    fn skip_space(&mut self) -> bool {
        let rest = self.rest();
        let len = rest.len() - rest.trim_start_matches(is_space).len();
        self.pos += len;
        len > 0
    }

    fn malformed(&self, what: &'static str, position: usize) -> Error {
        Error::malformed(what, at(position))
    }

    /// The document: an XML declaration, then the root element with
    /// comments, processing instructions and white space around it.
    fn document(&mut self) -> Result<()> {
        self.xml_declaration()?;
        let mut root = false;
        loop {
            if !self.open.is_empty() {
                self.content()?;
                continue;
            }
            self.skip_space();
            let rest = self.rest();
            if rest.is_empty() {
                break;
            }
            if rest.starts_with("<!--") {
                self.comment()?;
            } else if rest.starts_with("<?") {
                self.processing_instruction()?;
            } else if rest.starts_with("<!DOCTYPE") {
                return Err(self.malformed("the XML has a DOCTYPE", self.pos));
            } else if rest.starts_with('<') && !rest.starts_with("</") && !rest.starts_with("<!") {
                if root {
                    return Err(self.malformed("the XML has a second root element", self.pos));
                }
                root = true;
                self.start_tag()?;
            } else {
                return Err(self.malformed("text or markup outside the root element", self.pos));
            }
        }
        if !root {
            return Err(self.malformed("the XML has no element", self.pos));
        }
        Ok(())
    }

    /// The XML declaration, if the document starts with one: checked, as
    /// the reader reads only XML 1.0 in UTF-8.
    fn xml_declaration(&mut self) -> Result<()> {
        let rest = self.rest();
        if !rest.starts_with("<?xml") || !rest.get(5..).unwrap_or_default().starts_with(is_space) {
            return Ok(());
        }
        let start = self.pos;
        self.pos += "<?xml".len();
        self.skip_space();
        if !self.eat_str("version") {
            return Err(self.malformed("an XML declaration without its version", start));
        }
        let version = self.pseudo_attribute(start)?;
        let digits = version.strip_prefix("1.").unwrap_or_default();
        if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return Err(self.malformed("an XML declaration with a malformed version", start));
        }
        if version == "1.1" {
            return Err(self.malformed("the XML declaration names version 1.1", start));
        }
        let mut space = self.skip_space();
        if space && self.eat_str("encoding") {
            let encoding = self.pseudo_attribute(start)?;
            let mut chars = encoding.chars();
            let well_formed = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
            if !well_formed {
                return Err(
                    self.malformed("an XML declaration with a malformed encoding name", start)
                );
            }
            if !encoding.eq_ignore_ascii_case("UTF-8") {
                return Err(self.malformed(
                    "the XML declaration names an encoding other than UTF-8",
                    start,
                ));
            }
            space = self.skip_space();
        }
        if space && self.eat_str("standalone") {
            let standalone = self.pseudo_attribute(start)?;
            if standalone != "yes" && standalone != "no" {
                return Err(self.malformed("an XML declaration with a malformed standalone", start));
            }
            self.skip_space();
        }
        if !self.eat_str("?>") {
            return Err(self.malformed("a malformed XML declaration", start));
        }
        Ok(())
    }

    /// `= "value"` in the XML declaration, quoted either way; its value.
    fn pseudo_attribute(&mut self, start: usize) -> Result<&'a str> {
        self.skip_space();
        if !self.eat('=') {
            return Err(self.malformed("a malformed XML declaration", start));
        }
        self.skip_space();
        let quote = match self.peek() {
            Some(quote @ ('"' | '\'')) => quote,
            _ => return Err(self.malformed("a malformed XML declaration", start)),
        };
        self.pos += 1;
        let rest = self.rest();
        let len = rest
            .find(quote)
            .ok_or_else(|| self.malformed("a malformed XML declaration", start))?;
        self.pos += len + 1;
        Ok(rest.get(..len).unwrap_or_default())
    }

    /// One step inside the open element: a run of text, a reference, a
    /// CDATA section, a comment, a processing instruction, or a tag.
    fn content(&mut self) -> Result<()> {
        let rest = self.rest();
        if rest.is_empty() {
            let offset = self.open.last().map_or(0, |open| open.offset);
            return Err(self.malformed("an element is not closed", offset));
        }
        if rest.starts_with("</") {
            self.end_tag()
        } else if rest.starts_with("<!--") {
            self.comment()
        } else if rest.starts_with("<?") {
            self.processing_instruction()
        } else if rest.starts_with("<![CDATA[") {
            self.cdata()
        } else if rest.starts_with("<!DOCTYPE") {
            Err(self.malformed("the XML has a DOCTYPE", self.pos))
        } else if rest.starts_with("<!") {
            Err(self.malformed("a declaration XML does not allow here", self.pos))
        } else if rest.starts_with('<') {
            self.start_tag()
        } else if rest.starts_with('&') {
            let c = self.reference()?;
            self.run_char(c);
            Ok(())
        } else {
            let len = rest.find(['<', '&']).unwrap_or(rest.len());
            if let Some(i) = rest.get(..len).unwrap_or_default().find("]]>") {
                return Err(self.malformed("]]> in text", self.pos + i));
            }
            self.run_raw(self.pos, self.pos + len);
            self.pos += len;
            Ok(())
        }
    }

    /// A start tag or an empty-element tag, with its attributes and the
    /// namespaces it declares.
    fn start_tag(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += 1;
        let name = self.name()?;
        let mut raw = Vec::new();
        let empty = loop {
            let space = self.skip_space();
            if self.eat_str("/>") {
                break true;
            }
            if self.eat('>') {
                break false;
            }
            if self.rest().is_empty() {
                return Err(self.malformed("a start tag is not closed", start));
            }
            if !space {
                return Err(self.malformed("an attribute not preceded by white space", self.pos));
            }
            let offset = self.pos;
            let name = self.name()?;
            self.skip_space();
            if !self.eat('=') {
                return Err(self.malformed("an attribute without its value", offset));
            }
            self.skip_space();
            let value = self.attribute_value()?;
            raw.push(RawAttribute {
                name,
                value,
                offset,
            });
        };
        let mut written = HashSet::new();
        for attribute in &raw {
            if !written.insert(attribute.name) {
                return Err(self.malformed("an attribute repeated in one tag", attribute.offset));
            }
        }
        let declared = self.declared.len();
        for attribute in &raw {
            let (prefix, local) = split_name(attribute.name)
                .ok_or_else(|| self.malformed("a name with a misplaced colon", attribute.offset))?;
            if prefix == "xmlns" {
                self.declare(local, attribute.value.clone(), attribute.offset)?;
            } else if prefix.is_empty() && local == "xmlns" {
                self.declare("", attribute.value.clone(), attribute.offset)?;
            }
        }
        let (prefix, local) = split_name(name)
            .ok_or_else(|| self.malformed("a name with a misplaced colon", start))?;
        if prefix == "xmlns" {
            return Err(self.malformed("an element named with the prefix xmlns", start));
        }
        let namespace = self.resolve(prefix, true, start)?;
        let first = self.document.attributes.len();
        let mut resolved = HashSet::new();
        for attribute in raw {
            let (prefix, local) = split_name(attribute.name).unwrap_or_default();
            if prefix == "xmlns" || (prefix.is_empty() && local == "xmlns") {
                continue;
            }
            let namespace = self.resolve(prefix, false, attribute.offset)?;
            if !resolved.insert((namespace, local)) {
                return Err(self.malformed("an attribute repeated in one tag", attribute.offset));
            }
            self.document.attributes.push(StoredAttribute {
                namespace,
                local,
                value: attribute.value,
            });
        }
        if self.open.len() >= usize::try_from(self.max_depth).unwrap_or(usize::MAX) {
            return Err(Error::limit(
                "the XML nests deeper than the limit",
                at(start),
            ));
        }
        self.flush_text();
        let node = self.add(Kind::Element {
            offset: start,
            namespace,
            local,
            attributes: first..self.document.attributes.len(),
            first: NONE,
        });
        if empty {
            self.undeclare(declared);
        } else {
            self.open.push(Open {
                node,
                name,
                offset: start,
                declared,
                last: NONE,
            });
        }
        Ok(())
    }

    /// An end tag, which must repeat the name of the element it closes.
    fn end_tag(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += 2;
        let name = self.name()?;
        self.skip_space();
        if !self.eat('>') {
            return Err(self.malformed("an end tag is not closed", start));
        }
        if self.open.last().map(|open| open.name) != Some(name) {
            return Err(self.malformed("an end tag does not match its start tag", start));
        }
        self.flush_text();
        if let Some(open) = self.open.pop() {
            self.undeclare(open.declared);
        }
        Ok(())
    }

    /// Binds `prefix`, or the default namespace for `""`, to `uri`, by
    /// Namespaces in XML 1.0's rules (§3).
    fn declare(&mut self, prefix: &'a str, uri: Cow<'a, str>, offset: usize) -> Result<()> {
        if prefix == "xmlns" || uri == XMLNS {
            return Err(self.malformed("a declaration of the xmlns namespace", offset));
        }
        if (prefix == "xml") != (uri == namespaces::XML) {
            return Err(self.malformed("the xml prefix and the XML namespace bound apart", offset));
        }
        let id = if uri.is_empty() {
            if !prefix.is_empty() {
                return Err(self.malformed("a prefix declared with no namespace", offset));
            }
            NONE
        } else {
            self.intern(uri)
        };
        self.bindings.entry(prefix).or_default().push(id);
        self.declared.push(prefix);
        Ok(())
    }

    /// Ends the declarations made since `declared` were.
    fn undeclare(&mut self, declared: usize) {
        for prefix in self.declared.drain(declared..) {
            if let Some(stack) = self.bindings.get_mut(prefix) {
                stack.pop();
            }
        }
    }

    /// The namespace of `prefix`: for no prefix, the default namespace on
    /// an element, and none on an attribute.
    fn resolve(&self, prefix: &str, element: bool, offset: usize) -> Result<usize> {
        let bound = self.bindings.get(prefix).and_then(|stack| stack.last());
        match (prefix, bound) {
            ("", _) if !element => Ok(NONE),
            ("", bound) => Ok(bound.copied().unwrap_or(NONE)),
            (_, Some(&id)) => Ok(id),
            ("xml", None) => Ok(self.xml),
            (_, None) => Err(self.malformed("a prefix that is not declared", offset)),
        }
    }

    fn intern(&mut self, uri: Cow<'a, str>) -> usize {
        if let Some(&id) = self.ids.get(&uri) {
            return id;
        }
        let id = self.document.namespaces.len();
        self.document.namespaces.push(uri.clone());
        self.ids.insert(uri, id);
        id
    }

    /// A quoted attribute value, normalized: each reference replaced by
    /// its character, each tab, line end or space by one space.
    fn attribute_value(&mut self) -> Result<Cow<'a, str>> {
        let start = self.pos;
        let quote = match self.peek() {
            Some(quote @ ('"' | '\'')) => quote,
            _ => return Err(self.malformed("an attribute value not in quotes", start)),
        };
        self.pos += 1;
        let mut owned: Option<String> = None;
        let mut from = self.pos;
        loop {
            let Some(c) = self.peek() else {
                return Err(self.malformed("an attribute value is not closed", start));
            };
            let here = self.pos;
            match c {
                '<' => return Err(self.malformed("< in an attribute value", here)),
                '&' => {
                    let value = owned.get_or_insert_with(String::new);
                    value.push_str(self.text.get(from..here).unwrap_or_default());
                    let c = self.reference()?;
                    value.push(c);
                    from = self.pos;
                }
                '\t' | '\n' | '\r' => {
                    let value = owned.get_or_insert_with(String::new);
                    value.push_str(self.text.get(from..here).unwrap_or_default());
                    value.push(' ');
                    self.pos += 1;
                    if c == '\r' {
                        self.eat('\n');
                    }
                    from = self.pos;
                }
                _ if c == quote => {
                    self.pos += 1;
                    let tail = self.text.get(from..here).unwrap_or_default();
                    return Ok(match owned {
                        None => Cow::Borrowed(tail),
                        Some(mut value) => {
                            value.push_str(tail);
                            Cow::Owned(value)
                        }
                    });
                }
                _ => self.pos += c.len_utf8(),
            }
        }
    }

    /// A reference, `&name;` for one of the five predefined entities or
    /// `&#N;` and `&#xN;` for a character; the character it stands for.
    fn reference(&mut self) -> Result<char> {
        let start = self.pos;
        self.pos += 1;
        if self.eat('#') {
            let radix = if self.eat('x') { 16 } else { 10 };
            let digits = self.pos;
            let mut value: Option<u32> = Some(0);
            while let Some(digit) = self.peek().and_then(|c| c.to_digit(radix)) {
                value = value
                    .and_then(|v| v.checked_mul(radix))
                    .and_then(|v| v.checked_add(digit));
                self.pos += 1;
            }
            if self.pos == digits || !self.eat(';') {
                return Err(self.malformed("a malformed character reference", start));
            }
            return value
                .and_then(char::from_u32)
                .filter(|&c| is_char(c))
                .ok_or_else(|| {
                    self.malformed("a reference to a character XML does not allow", start)
                });
        }
        let name = self.name()?;
        if !self.eat(';') {
            return Err(self.malformed("an entity reference without its ;", start));
        }
        match name {
            "lt" => Ok('<'),
            "gt" => Ok('>'),
            "amp" => Ok('&'),
            "apos" => Ok('\''),
            "quot" => Ok('"'),
            _ => Err(self.malformed("an entity that is not predefined", start)),
        }
    }

    /// A comment, skipped: `--` may not occur inside it.
    fn comment(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += "<!--".len();
        let rest = self.rest();
        let dashes = rest
            .find("--")
            .ok_or_else(|| self.malformed("a comment is not closed", start))?;
        if !rest.get(dashes..).unwrap_or_default().starts_with("-->") {
            return Err(self.malformed("-- inside a comment", self.pos + dashes));
        }
        self.pos += dashes + "-->".len();
        Ok(())
    }

    /// A processing instruction, skipped; its target may not be `xml`,
    /// in any case, which only the XML declaration at the start may be.
    fn processing_instruction(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += "<?".len();
        let target = self.name()?;
        if target.eq_ignore_ascii_case("xml") {
            return Err(self.malformed("an XML declaration not at the start", start));
        }
        if target.contains(':') {
            return Err(self.malformed("a name with a misplaced colon", start));
        }
        if self.eat_str("?>") {
            return Ok(());
        }
        if !self.skip_space() {
            return Err(self.malformed("a malformed processing instruction", start));
        }
        let len = self
            .rest()
            .find("?>")
            .ok_or_else(|| self.malformed("a processing instruction is not closed", start))?;
        self.pos += len + "?>".len();
        Ok(())
    }

    /// A CDATA section: its text kept as written, line ends normalized.
    fn cdata(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += "<![CDATA[".len();
        let len = self
            .rest()
            .find("]]>")
            .ok_or_else(|| self.malformed("a CDATA section is not closed", start))?;
        self.run_raw(self.pos, self.pos + len);
        self.pos += len + "]]>".len();
        Ok(())
    }

    /// A name, by XML 1.0's rules for its first character and the rest
    /// (§2.3); a colon is checked by the caller.
    fn name(&mut self) -> Result<&'a str> {
        let rest = self.rest();
        if !rest.chars().next().is_some_and(is_name_start) {
            return Err(self.malformed("a name expected", self.pos));
        }
        let len = rest
            .char_indices()
            .find(|&(_, c)| !is_name_char(c))
            .map_or(rest.len(), |(i, _)| i);
        self.pos += len;
        Ok(rest.get(..len).unwrap_or_default())
    }

    /// Adds the text of `start..end` to the run, line ends normalized.
    fn run_raw(&mut self, start: usize, end: usize) {
        let mut from = start;
        while let Some(i) = self.text.get(from..end).and_then(|s| s.find('\r')) {
            let cr = from + i;
            self.run_slice(from, cr);
            self.run_char('\n');
            from = cr + 1;
            if self
                .text
                .get(from..end)
                .is_some_and(|s| s.starts_with('\n'))
            {
                from += 1;
            }
        }
        self.run_slice(from, end);
    }

    fn run_slice(&mut self, start: usize, end: usize) {
        if start == end {
            return;
        }
        self.run = match std::mem::replace(&mut self.run, Run::Empty) {
            Run::Empty => Run::Slice(start..end),
            Run::Slice(range) if range.end == start => Run::Slice(range.start..end),
            Run::Slice(range) => {
                let mut owned = String::from(self.text.get(range).unwrap_or_default());
                owned.push_str(self.text.get(start..end).unwrap_or_default());
                Run::Owned(owned)
            }
            Run::Owned(mut owned) => {
                owned.push_str(self.text.get(start..end).unwrap_or_default());
                Run::Owned(owned)
            }
        };
    }

    fn run_char(&mut self, c: char) {
        self.run = match std::mem::replace(&mut self.run, Run::Empty) {
            Run::Empty => Run::Owned(String::from(c)),
            Run::Slice(range) => {
                let mut owned = String::from(self.text.get(range).unwrap_or_default());
                owned.push(c);
                Run::Owned(owned)
            }
            Run::Owned(mut owned) => {
                owned.push(c);
                Run::Owned(owned)
            }
        };
    }

    /// Adds the text read since the last child to the open element.
    fn flush_text(&mut self) {
        let text = match std::mem::replace(&mut self.run, Run::Empty) {
            Run::Empty => return,
            Run::Slice(range) => Cow::Borrowed(self.text.get(range).unwrap_or_default()),
            Run::Owned(owned) => Cow::Owned(owned),
        };
        if !self.open.is_empty() {
            self.add(Kind::Text(text));
        }
    }

    /// Adds a node as the last child of the open element, or as the root.
    fn add(&mut self, kind: Kind<'a>) -> usize {
        let index = self.document.nodes.len();
        let parent = self.open.last().map_or(NONE, |open| open.node);
        self.document.nodes.push(Node {
            parent,
            next: NONE,
            kind,
        });
        if let Some(open) = self.open.last_mut() {
            let previous = std::mem::replace(&mut open.last, index);
            let link = if previous == NONE {
                match self
                    .document
                    .nodes
                    .get_mut(open.node)
                    .map(|node| &mut node.kind)
                {
                    Some(Kind::Element { first, .. }) => Some(first),
                    _ => None,
                }
            } else {
                self.document
                    .nodes
                    .get_mut(previous)
                    .map(|node| &mut node.next)
            };
            if let Some(link) = link {
                *link = index;
            }
        }
        index
    }
}

/// A name split at its colon into a prefix and a local name, the prefix
/// empty when there is no colon.
///
/// `None` when the colon is not where Namespaces in XML allows one: once,
/// between two parts that are not empty.
fn split_name(name: &str) -> Option<(&str, &str)> {
    match name.split_once(':') {
        None => Some(("", name)),
        Some((prefix, local))
            if !prefix.is_empty()
                && !local.contains(':')
                && local.chars().next().is_some_and(is_name_start) =>
        {
            Some((prefix, local))
        }
        Some(_) => None,
    }
}

/// White space as XML counts it (§2.3, `S`).
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// A character XML allows in a document (§2.2, `Char`).
fn is_char(c: char) -> bool {
    matches!(c,
        '\t' | '\n' | '\r' | '\u{20}'..='\u{D7FF}' | '\u{E000}'..='\u{FFFD}' | '\u{10000}'..='\u{10FFFF}')
}

/// A character that may start a name (§2.3, `NameStartChar`).
fn is_name_start(c: char) -> bool {
    matches!(c,
        ':' | 'A'..='Z' | '_' | 'a'..='z'
        | '\u{C0}'..='\u{D6}' | '\u{D8}'..='\u{F6}' | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}' | '\u{37F}'..='\u{1FFF}' | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}' | '\u{2C00}'..='\u{2FEF}' | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}' | '\u{FDF0}'..='\u{FFFD}' | '\u{10000}'..='\u{EFFFF}')
}

/// A character that may follow the first in a name (§2.3, `NameChar`).
fn is_name_char(c: char) -> bool {
    is_name_start(c)
        || matches!(c,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    fn read(text: &str) -> Document<'_> {
        parse(text.as_bytes(), &Limits::default()).expect("well-formed XML")
    }

    /// The error `bytes` give, which must be damage.
    fn damage(bytes: &[u8]) -> Error {
        let err = parse(bytes, &Limits::default()).err().expect("damage");
        assert_eq!(err.kind(), ErrorKind::Malformed, "{err}");
        err
    }

    /// The runs of text of the root element, joined.
    fn text(document: &Document<'_>) -> String {
        document
            .root()
            .children()
            .filter_map(|child| match child {
                Child::Text(run) => Some(run),
                Child::Element(_) => None,
            })
            .collect()
    }

    /// The child elements of `element`.
    fn elements(element: Element<'_>) -> Vec<Element<'_>> {
        element
            .children()
            .filter_map(|child| match child {
                Child::Element(element) => Some(element),
                Child::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn the_predefined_entities_and_both_reference_forms_give_their_characters() {
        let document = read(
            "<a b='&lt;&gt;&amp;&apos;&quot;&#65;&#x42;'>&lt;&gt;&amp;&apos;&quot;&#65;&#x42;&#x1F600;</a>",
        );
        assert_eq!(text(&document), "<>&'\"AB\u{1F600}");
        assert_eq!(document.root().attribute(None, "b"), Some("<>&'\"AB"));
    }

    #[test]
    fn a_reference_to_a_character_xml_does_not_allow_is_damage() {
        for packet in [
            "<a>&#0;</a>",
            "<a>&#x0;</a>",
            "<a>&#x110000;</a>",
            "<a>&#xD800;</a>",
            "<a>&#xFFFE;</a>",
            "<a>&#99999999999999999999;</a>",
        ] {
            let err = damage(packet.as_bytes());
            assert_eq!(err.offset(), Some(3), "{packet}");
        }
        let err = damage(b"<a b='&#1;'/>");
        assert_eq!(err.offset(), Some(6));
    }

    #[test]
    fn a_malformed_reference_is_damage() {
        for packet in [
            "<a>&#;</a>",
            "<a>&#x;</a>",
            "<a>&#X41;</a>",
            "<a>&#65</a>",
            "<a>&lt</a>",
            "<a>& b</a>",
        ] {
            damage(packet.as_bytes());
        }
    }

    #[test]
    fn an_entity_that_is_not_predefined_is_damage() {
        let err = damage(b"<a>caf&eacute;</a>");
        assert_eq!(
            err.to_string(),
            "an entity that is not predefined at byte 6"
        );
        damage(b"<a b='&nbsp;'/>");
    }

    #[test]
    fn cdata_keeps_its_text_as_written() {
        let document = read("<a>x <![CDATA[< y ]] & z]]> w</a>");
        assert_eq!(text(&document), "x < y ]] & z w");
        assert_eq!(elements(document.root()).len(), 0);
        damage(b"<a><![CDATA[never closed</a>");
        damage(b"<![CDATA[outside]]><a/>");
    }

    #[test]
    fn comments_and_processing_instructions_are_skipped_anywhere() {
        let document = read(
            "<?pi before?><!-- before --><a><!-- c --><b/><?pi data?><c/>t<!--x-->u<?p?>v</a><!-- after --><?pi after?>",
        );
        let names: Vec<&str> = elements(document.root())
            .iter()
            .map(|e| e.local())
            .collect();
        assert_eq!(names, ["b", "c"]);
        assert_eq!(text(&document), "tuv");
    }

    #[test]
    fn a_comment_holding_two_dashes_or_never_closed_is_damage() {
        damage(b"<a><!-- a -- b --></a>");
        damage(b"<a><!-- ends with three ---></a>");
        damage(b"<a><!-- never closed</a>");
        read("<a><!----></a>");
    }

    #[test]
    fn a_processing_instruction_named_xml_past_the_start_is_damage() {
        damage(b"<a><?xml version='1.0'?></a>");
        damage(b"<!-- c --><?xml version='1.0'?><a/>");
        damage(b"<a><?XmL x?></a>");
        damage(b"<a><?never closed</a>");
    }

    #[test]
    fn line_ends_become_lf_in_text_and_spaces_in_attributes() {
        let document = read("<a b='1\r\n2\r3\n4'>1\r\n2\r3\n4<![CDATA[\r\n5\r]]></a>");
        assert_eq!(text(&document), "1\n2\n3\n4\n5\n");
        assert_eq!(document.root().attribute(None, "b"), Some("1 2 3 4"));
    }

    #[test]
    fn a_tab_or_newline_in_an_attribute_becomes_a_space_but_not_through_a_reference() {
        let document = read("<a b='x\ty\nz  w' c='x&#9;y&#10;z'/>");
        assert_eq!(document.root().attribute(None, "b"), Some("x y z  w"));
        assert_eq!(document.root().attribute(None, "c"), Some("x\ty\nz"));
    }

    #[test]
    fn attributes_take_either_quote() {
        let document = read(r#"<a b='say "hi"' c="it's"/>"#);
        assert_eq!(document.root().attribute(None, "b"), Some("say \"hi\""));
        assert_eq!(document.root().attribute(None, "c"), Some("it's"));
        damage(b"<a b=1/>");
        damage(b"<a b='1/>");
    }

    #[test]
    fn a_leading_byte_order_mark_is_skipped() {
        let document = read("\u{FEFF}<a>x</a>");
        assert_eq!(document.root().local(), "a");
        assert_eq!(text(&document), "x");
        read("\u{FEFF}<?xml version='1.0' encoding='UTF-8'?><a/>");
    }

    #[test]
    fn a_name_may_hold_letters_outside_ascii() {
        let document = read("<é:ö xmlns:é='urn:x' é:ü='1'/>");
        let root = document.root();
        assert_eq!(root.namespace(), Some("urn:x"));
        assert_eq!(root.local(), "ö");
        assert_eq!(root.attribute(Some("urn:x"), "ü"), Some("1"));
        // U+00D7, the multiplication sign, is no letter.
        damage("<a\u{D7}b/>".as_bytes());
        damage(b"<1a/>");
    }

    #[test]
    fn bytes_that_are_not_utf8_are_damage_at_their_offset() {
        let err = damage(b"<a>caf\xE9</a>");
        assert_eq!(err.to_string(), "the XML is not UTF-8 at byte 6");
    }

    #[test]
    fn utf16_and_utf32_are_damage() {
        let utf16le: Vec<u8> = "\u{FEFF}<a/>"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let utf16be: Vec<u8> = "<a/>".encode_utf16().flat_map(u16::to_be_bytes).collect();
        let utf32be: Vec<u8> = "<a/>"
            .chars()
            .flat_map(|c| u32::from(c).to_be_bytes())
            .collect();
        for bytes in [utf16le, utf16be, utf32be] {
            let err = damage(&bytes);
            assert_eq!(
                err.to_string(),
                "the XML is in UTF-16 or UTF-32, not UTF-8 at byte 0"
            );
        }
    }

    #[test]
    fn a_character_xml_does_not_allow_is_damage() {
        let err = damage(b"<a>\x01</a>");
        assert_eq!(err.offset(), Some(3));
        damage(b"<a>\0</a>");
        damage("<a>\u{FFFF}</a>".as_bytes());
    }

    #[test]
    fn an_xml_declaration_must_name_utf8_and_xml_1_0() {
        read("<?xml version=\"1.0\" encoding=\"UTF-8\"?><a/>");
        read("<?xml version='1.0' encoding='utf-8' standalone='yes'?>\n<a/>");
        read("<?xml version = '1.0' ?><a/>");
        // A later 1.x version is read as 1.0 (XML 1.0, section 2.8).
        read("<?xml version='1.2'?><a/>");
        for (packet, message) in [
            (
                "<?xml version='1.0' encoding='ISO-8859-1'?><a/>",
                "the XML declaration names an encoding other than UTF-8",
            ),
            (
                "<?xml version='1.0' encoding='UTF-16'?><a/>",
                "the XML declaration names an encoding other than UTF-8",
            ),
            (
                "<?xml version='1.1'?><a/>",
                "the XML declaration names version 1.1",
            ),
            (
                "<?xml version='2.0'?><a/>",
                "an XML declaration with a malformed version",
            ),
            (
                "<?xml encoding='UTF-8'?><a/>",
                "an XML declaration without its version",
            ),
            (
                "<?xml version='1.0' standalone='maybe'?><a/>",
                "an XML declaration with a malformed standalone",
            ),
            (
                "<?xml version='1.0' encoding='8bit'?><a/>",
                "an XML declaration with a malformed encoding name",
            ),
            (
                "<?xml version='1.0'encoding='UTF-8'?><a/>",
                "a malformed XML declaration",
            ),
        ] {
            let err = damage(packet.as_bytes());
            assert_eq!(err.to_string(), format!("{message} at byte 0"), "{packet}");
        }
    }

    #[test]
    fn a_doctype_is_damage() {
        let err = damage(b"<!DOCTYPE a [<!ENTITY e 'x'>]><a>&e;</a>");
        assert_eq!(err.to_string(), "the XML has a DOCTYPE at byte 0");
        damage(b"<a><!DOCTYPE a></a>");
        damage(b"<a><!ELEMENT a ANY></a>");
    }

    #[test]
    fn an_undeclared_prefix_is_damage() {
        // As Sony Ericsson phones write `cell:`.
        let err = damage(b"<x:xmpmeta xmlns:x='adobe:ns:meta/'><cell:Model/></x:xmpmeta>");
        assert_eq!(err.to_string(), "a prefix that is not declared at byte 36");
        damage(b"<a cell:b='1'/>");
        damage(b"<p:a xmlns:p='urn:1'/><p:b/>");
    }

    #[test]
    fn a_default_namespace_holds_elements_but_not_attributes() {
        let document = read("<a xmlns='urn:d' c='1'><b/></a>");
        let root = document.root();
        assert!(root.is("urn:d", "a"));
        assert_eq!(root.attribute(None, "c"), Some("1"));
        assert_eq!(root.attribute(Some("urn:d"), "c"), None);
        assert!(elements(root)[0].is("urn:d", "b"));
    }

    #[test]
    fn a_prefix_declared_again_in_a_child_holds_only_there() {
        let document = read("<p:a xmlns:p='urn:1'><p:b xmlns:p='urn:2'/><p:c/></p:a>");
        let children = elements(document.root());
        assert!(children[0].is("urn:2", "b"));
        assert!(children[1].is("urn:1", "c"));
        assert!(document.root().is("urn:1", "a"));
    }

    #[test]
    fn an_empty_default_namespace_undeclares_it() {
        let document = read("<a xmlns='urn:d'><b xmlns=''><c/></b><d/></a>");
        let children = elements(document.root());
        assert_eq!(children[0].namespace(), None);
        assert_eq!(elements(children[0])[0].namespace(), None);
        assert_eq!(children[1].namespace(), Some("urn:d"));
    }

    #[test]
    fn the_xml_prefix_needs_no_declaration() {
        let document = read("<a xml:lang='en'/>");
        assert_eq!(
            document.root().attribute(Some(namespaces::XML), "lang"),
            Some("en")
        );
        read("<a xmlns:xml='http://www.w3.org/XML/1998/namespace' xml:lang='en'/>");
    }

    #[test]
    fn a_declaration_namespaces_in_xml_forbids_is_damage() {
        for packet in [
            "<a xmlns:p=''/>",
            "<a xmlns:xml='urn:x'/>",
            "<a xmlns:p='http://www.w3.org/XML/1998/namespace'/>",
            "<a xmlns='http://www.w3.org/XML/1998/namespace'/>",
            "<a xmlns:xmlns='urn:x'/>",
            "<a xmlns:p='http://www.w3.org/2000/xmlns/'/>",
            "<xmlns:a/>",
        ] {
            damage(packet.as_bytes());
        }
    }

    #[test]
    fn an_attribute_repeated_after_its_prefix_is_resolved_is_damage() {
        let err = damage(b"<a xmlns:p='urn:1' xmlns:q='urn:1' p:b='1' q:b='2'/>");
        assert_eq!(
            err.to_string(),
            "an attribute repeated in one tag at byte 43"
        );
        damage(b"<a b='1' b='2'/>");
        damage(b"<a xmlns:p='urn:1' xmlns:p='urn:2'/>");
        // The same local name in two namespaces, or in none and one, is two
        // attributes.
        read("<a xmlns:p='urn:1' xmlns:q='urn:2' p:b='1' q:b='2' b='3'/>");
    }

    #[test]
    fn a_less_than_sign_in_an_attribute_is_damage() {
        let err = damage(b"<a b='x<y'/>");
        assert_eq!(err.to_string(), "< in an attribute value at byte 7");
        read("<a b='x&lt;y &gt; z'/>");
    }

    #[test]
    fn a_name_with_a_misplaced_colon_is_damage() {
        for packet in [
            "<a:b:c xmlns:a='urn:1'/>",
            "<:a/>",
            "<a: />",
            "<a xmlns:p='urn:1' p:b:c='1'/>",
            "<a xmlns:p:q='urn:1'/>",
            "<p:1 xmlns:p='urn:1'/>",
        ] {
            let err = damage(packet.as_bytes());
            assert_eq!(
                err.to_string().split(" at byte").next(),
                Some("a name with a misplaced colon"),
                "{packet}"
            );
        }
    }

    #[test]
    fn elements_nest_to_the_limit_on_depth_and_no_deeper() {
        let limits = Limits::default().with_max_xml_depth(3);
        assert!(parse(b"<a><b><c/></b></a>", &limits).is_ok());
        let err = parse(b"<a><b><c><d/></c></b></a>", &limits)
            .err()
            .expect("past the limit");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
        assert_eq!(
            err.to_string(),
            "the XML nests deeper than the limit at byte 9"
        );
        // 64 levels by default.
        let deep = format!("{}{}", "<a>".repeat(64), "</a>".repeat(64));
        read(&deep);
        let deeper = format!("{}{}", "<a>".repeat(65), "</a>".repeat(65));
        let err = parse(deeper.as_bytes(), &Limits::default())
            .err()
            .expect("past the limit");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
    }

    #[test]
    fn an_element_not_closed_at_the_end_is_damage() {
        let err = damage(b"<a><b></b><c>");
        assert_eq!(err.to_string(), "an element is not closed at byte 10");
        damage(b"<a");
        damage(b"<a b='1'");
    }

    #[test]
    fn nuls_and_spaces_after_the_packet_wrapper_are_padding() {
        // A Z 9 pads its packet to 32,768 bytes with spaces inside the
        // wrapper. NULs after the wrapper, which a value in a file may
        // hold past the packet's text, are no part of the XML either.
        let mut packet = "<?xpacket begin='\u{FEFF}' id='W5M0MpCehiHzreSzNTczkc9d'?>\n<a>x</a>\n"
            .to_owned()
            .into_bytes();
        packet.extend_from_slice(&[b' '; 300]);
        packet.extend_from_slice(b"\n<?xpacket end='w'?>");
        packet.extend_from_slice(&[0; 200]);
        let document = parse(&packet, &Limits::default()).expect("padded packet");
        assert_eq!(text(&document), "x");
        // A NUL inside the packet is no padding.
        damage(b"<a>x</a>\0 ");
        // NULs alone are an empty packet, not UTF-16.
        let err = damage(b"\0\0\0");
        assert_eq!(err.to_string(), "the XML has no element at byte 0");
    }

    #[test]
    fn an_end_tag_that_does_not_match_is_damage() {
        let err = damage(b"<a><b></a></b>");
        assert_eq!(
            err.to_string(),
            "an end tag does not match its start tag at byte 6"
        );
        damage(b"<p:a xmlns:p='urn:1' xmlns:q='urn:1'></q:a>");
        damage(b"<a></a ");
    }

    #[test]
    fn anything_but_markup_outside_the_root_is_damage() {
        for packet in [
            "<a/><b/>",
            "<a/>x",
            "x<a/>",
            "</a>",
            "",
            "   ",
            "<!-- only a comment -->",
        ] {
            damage(packet.as_bytes());
        }
    }

    #[test]
    fn a_closing_cdata_delimiter_in_text_is_damage() {
        damage(b"<a>x ]]> y</a>");
        read("<a>x ]] > y</a>");
    }

    #[test]
    fn attributes_must_be_apart_and_have_values() {
        damage(b"<a b='1'c='2'/>");
        damage(b"<a b/>");
        read("<a b = '1'\n\tc='2' />");
    }

    #[test]
    fn text_between_elements_is_kept_in_order() {
        let document = read("<a>x<b>y</b>z</a>");
        let children: Vec<String> = document
            .root()
            .children()
            .map(|child| match child {
                Child::Text(run) => format!("text {run}"),
                Child::Element(element) => format!("element {}", element.local()),
            })
            .collect();
        assert_eq!(children, ["text x", "element b", "text z"]);
        let b = elements(document.root())[0];
        assert_eq!(b.parent().map(Element::local), Some("a"));
        assert!(document.root().parent().is_none());
        let names: Vec<&str> = document.elements().map(Element::local).collect();
        assert_eq!(names, ["a", "b"]);
    }

    #[test]
    fn every_cut_of_a_packet_parses_or_fails_without_panicking() {
        let packet = "\u{FEFF}<?xml version='1.0' encoding='UTF-8'?>\
            <x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='t'><!-- c -->\
            <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>\
            <rdf:Description rdf:about='' xmlns:dc='http://purl.org/dc/elements/1.1/'>\
            <dc:title><rdf:Alt><rdf:li xml:lang='x-default'>&lt;caf&#xE9;&gt;\r\n<![CDATA[]]]]></rdf:li>\
            </rdf:Alt></dc:title><?pi x?></rdf:Description></rdf:RDF></x:xmpmeta>";
        let bytes = packet.as_bytes();
        assert!(parse(bytes, &Limits::default()).is_ok());
        for len in 0..bytes.len() {
            let _ = parse(&bytes[..len], &Limits::default());
        }
    }
}
