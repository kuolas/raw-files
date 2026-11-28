// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF walk: the XMP data model read from the tree the XML reader
//! gives, by the rules of XMP Part 1, §7.
//!
//! A packet is RDF in XML. Its properties lie in `rdf:Description`
//! elements inside one `rdf:RDF` element, which an `x:xmpmeta` element
//! and any other XML may surround (§7.3). The walk takes the first
//! `rdf:RDF` in the document, wherever it lies, and the toolkit's name
//! from the `x:xmpmeta` around it. Every `rdf:Description` directly
//! inside `rdf:RDF` adds its properties to the one set, in document order
//! (§7.4).
//!
//! A property is an attribute of an `rdf:Description`, a simple value, or
//! one of its child elements. The value of an element, whether a
//! property, a field of a structure or an item of an array, takes one of
//! these forms:
//!
//! - text alone is a simple value (§7.5), the text as written, white
//!   space included;
//! - an `rdf:resource` attribute and no content is a URI (§7.5);
//! - an `rdf:parseType="Resource"` attribute makes a structure of the
//!   element's own children (§7.9.2.3);
//! - attributes alone, with no content, make a structure of simple fields
//!   (§7.9.2.4);
//! - one `rdf:Description` element makes a structure of its attributes and
//!   its children (§7.6, §7.9.2.2);
//! - one `rdf:Bag`, `rdf:Seq` or `rdf:Alt` element makes an unordered,
//!   ordered or alternative array of its `rdf:li` children (§7.7).
//!
//! A field or an item takes any of these forms in turn, so values nest
//! as deep as the XML does.
//!
//! Damage here has a smaller reach than in the XML. A property whose RDF
//! breaks XMP Part 1, §7, anywhere inside its value, is left out, and the
//! others stand: `rdf:parseType="Literal"`, `rdf:ID`, `rdf:nodeID`,
//! `rdf:li` outside an array, or text beside an element, for example. So
//! is a
//! property whose name is in the RDF namespace, which XMP forbids but for
//! `rdf:type`, or the XML namespace, or no namespace at all: an XMP name
//! needs a namespace (§6.2). An `rdf:Description` that breaks §7.4
//! itself, with text in it, an `xml:lang`, or an RDF attribute other than
//! `rdf:about` and `rdf:type`, is left out with every property it holds.
//! A property repeated in the packet, or a field in its structure, is left
//! out after its first. An
//! attribute with no namespace is not an XMP name, and is ignored, as an
//! attribute of the XML namespace other than `xml:lang` is.
//!
//! The walk is a loop, never recursive. It counts every value it builds,
//! and stops with [`ErrorKind::LimitExceeded`] past
//! [`Limits::max_xmp_values`].
//!
//! [`ErrorKind::LimitExceeded`]: crate::ErrorKind::LimitExceeded

use std::collections::HashSet;

use super::{Xmp, XmpArrayKind, XmpForm, XmpProperty, XmpValue};
use crate::error::{Error, Result};
use crate::limits::Limits;
use crate::namespaces::{RDF, XML};
use crate::xml::{self, Attribute, Child, Children, Document, Element};

/// The namespace of the `x:xmpmeta` element (XMP Part 1, §7.3.3).
const XMP_META: &str = "adobe:ns:meta/";

/// The properties of the packet `document` holds, read under `limits`.
///
/// # Errors
///
/// [`Malformed`](crate::ErrorKind::Malformed) when the document holds no
/// `rdf:RDF` element; [`LimitExceeded`](crate::ErrorKind::LimitExceeded)
/// past [`Limits::max_xmp_values`].
pub(super) fn read(document: &Document<'_>, limits: &Limits) -> Result<Xmp> {
    let rdf = document
        .elements()
        .find(|element| element.is(RDF, "RDF"))
        .ok_or_else(|| {
            Error::malformed(
                "the XML holds no rdf:RDF element",
                at(document.root().offset()),
            )
        })?;
    let mut walk = Walk {
        values: 0,
        max: limits.max_xmp_values(),
        properties: Vec::new(),
        names: HashSet::new(),
        about: None,
    };
    for child in rdf.children() {
        if let Child::Element(description) = child
            && description.is(RDF, "Description")
        {
            walk.description(description)?;
        }
    }
    Ok(Xmp {
        properties: walk.properties,
        toolkit: toolkit(rdf),
        about: walk.about,
    })
}

/// The `x:xmptk` attribute of the nearest `x:xmpmeta` element around
/// `rdf`; `None` when there is none, or it is empty.
fn toolkit(rdf: Element<'_>) -> Option<String> {
    let mut around = rdf.parent();
    while let Some(element) = around {
        if element.is(XMP_META, "xmpmeta") {
            return element
                .attribute(Some(XMP_META), "xmptk")
                .filter(|toolkit| !toolkit.is_empty())
                .map(str::to_owned);
        }
        around = element.parent();
    }
    None
}

/// A position in the packet as an error's byte offset.
fn at(position: usize) -> u64 {
    u64::try_from(position).unwrap_or(u64::MAX)
}

/// The walk over one packet's property set.
struct Walk<'d> {
    /// The values built so far, and the most allowed.
    values: u32,
    max: u32,
    properties: Vec<XmpProperty>,
    /// The name of each property in `properties`, to leave a repeat out.
    names: HashSet<(&'d str, &'d str)>,
    about: Option<String>,
}

impl<'d> Walk<'d> {
    /// Counts `value`, built from the element at `offset`, against the
    /// cap on values.
    fn count(&mut self, value: XmpValue, offset: usize) -> Result<XmpValue> {
        if self.values >= self.max {
            return Err(Error::limit(
                "the XMP packet gives more values than the limit",
                at(offset),
            ));
        }
        self.values += 1;
        Ok(value)
    }

    /// Adds the properties of a top-level `rdf:Description`, unless it
    /// breaks XMP Part 1, §7.4.
    fn description(&mut self, description: Element<'d>) -> Result<()> {
        if !is_property_set(description) {
            return Ok(());
        }
        for attribute in description.attributes() {
            match (attribute.namespace, attribute.local) {
                (Some(RDF), "about") => {
                    if self.about.is_none() && !attribute.value.is_empty() {
                        self.about = Some(attribute.value.to_owned());
                    }
                }
                (None | Some(XML), _) => {}
                (Some(namespace), local) => {
                    let value = self.count(attribute_value(attribute), description.offset())?;
                    self.add(namespace, local, value);
                }
            }
        }
        for child in description.children() {
            if let Child::Element(element) = child
                && let Some(namespace) = property_namespace(element)
                && let Some(value) = self.property(element)?
            {
                self.add(namespace, element.local(), value);
            }
        }
        Ok(())
    }

    /// Adds a property, unless the packet gave its name before.
    fn add(&mut self, namespace: &'d str, local: &'d str, value: XmpValue) {
        if self.names.insert((namespace, local)) {
            self.properties
                .push(XmpProperty::new(namespace, local, value));
        }
    }

    /// The value of a property element. `None` when it is left out: its
    /// RDF, or that of a value inside it, breaks XMP Part 1, §7.
    ///
    /// The walk goes down the value's elements without recursing. It keeps
    /// a stack of the structures and arrays still open. Each element is
    /// opened, and a value read whole goes to the structure or array that
    /// holds it. That one is read whole once its last child has gone to
    /// it.
    fn property(&mut self, element: Element<'d>) -> Result<Option<XmpValue>> {
        let mut open: Vec<Frame<'d>> = Vec::new();
        let mut next = Some(element);
        loop {
            let (element, value) = if let Some(element) = next.take() {
                match self.open(element)? {
                    Opened::Broken => return Ok(None),
                    Opened::Frame(frame) => {
                        open.push(frame);
                        continue;
                    }
                    Opened::Value(value) => (element, value),
                }
            } else {
                let Some(frame) = open.last_mut() else {
                    return Ok(None);
                };
                match frame.children.next() {
                    Some(Child::Text(run)) if xml::is_blank(run) => continue,
                    Some(Child::Text(_)) => return Ok(None),
                    Some(Child::Element(child)) if frame.allows(child) => {
                        next = Some(child);
                        continue;
                    }
                    Some(Child::Element(_)) => return Ok(None),
                    None => {
                        let Some(frame) = open.pop() else {
                            return Ok(None);
                        };
                        let element = frame.element;
                        (element, self.count(frame.finish(), element.offset())?)
                    }
                }
            };
            match open.last_mut() {
                None => return Ok(Some(value)),
                Some(frame) => {
                    if !frame.take(element, value) {
                        return Ok(None);
                    }
                }
            }
        }
    }

    /// What a property element holds, from its attributes and content:
    /// a value read whole, a structure or array whose elements are read
    /// next, or RDF that XMP Part 1 forbids.
    fn open(&mut self, element: Element<'d>) -> Result<Opened<'d>> {
        let mut resource = None;
        let mut parse_type = None;
        let mut fields = Vec::new();
        for attribute in element.attributes() {
            match (attribute.namespace, attribute.local) {
                (Some(RDF), "resource") => resource = Some(attribute.value),
                (Some(RDF), "parseType") => parse_type = Some(attribute.value),
                (Some(RDF), "type") => fields.push(attribute),
                (Some(RDF), _) => return Ok(Opened::Broken),
                // A qualifier, read by a later part of the walk.
                (Some(XML), "lang") => return Ok(Opened::Broken),
                (None | Some(XML), _) => {}
                (Some(_), _) => fields.push(attribute),
            }
        }
        let mut first = None;
        let mut more = false;
        let mut text = String::new();
        for child in element.children() {
            match child {
                Child::Element(child) if first.is_none() => first = Some(child),
                Child::Element(_) => more = true,
                Child::Text(run) => text.push_str(run),
            }
        }
        let blank = xml::is_blank(&text);
        if let Some(parse_type) = parse_type {
            // A structure whose fields are the element's own children
            // (section 7.9.2.3); no field may be an attribute then.
            if parse_type != "Resource" || resource.is_some() || !fields.is_empty() {
                return Ok(Opened::Broken);
            }
            return Ok(Opened::Frame(Frame::fields(element, element.children())));
        }
        if let Some(uri) = resource {
            if !fields.is_empty() || first.is_some() || !blank {
                return Ok(Opened::Broken);
            }
            let value = self.count(XmpValue::uri(uri), element.offset())?;
            return Ok(Opened::Value(value));
        }
        if !fields.is_empty() {
            // A structure whose fields are all attributes, with no content
            // (section 7.9.2.4).
            if first.is_some() || !blank {
                return Ok(Opened::Broken);
            }
            let mut structure = Vec::with_capacity(fields.len());
            for field in fields {
                let value = self.count(XmpValue::simple(field.value), element.offset())?;
                let namespace = field.namespace.unwrap_or_default();
                structure.push(XmpProperty::new(namespace, field.local, value));
            }
            let value = self.count(
                XmpValue::of(XmpForm::Structure(structure)),
                element.offset(),
            )?;
            return Ok(Opened::Value(value));
        }
        let Some(child) = first else {
            let value = self.count(XmpValue::simple(text), element.offset())?;
            return Ok(Opened::Value(value));
        };
        // One element and no text beside it: an array, or a structure in
        // an rdf:Description (sections 7.6 and 7.7).
        if more || !blank {
            return Ok(Opened::Broken);
        }
        let kind = match (child.namespace(), child.local()) {
            (Some(RDF), "Bag") => XmpArrayKind::Unordered,
            (Some(RDF), "Seq") => XmpArrayKind::Ordered,
            (Some(RDF), "Alt") => XmpArrayKind::Alternative,
            (Some(RDF), "Description") => return self.description_value(element, child),
            // A typed node, read by a later part of the walk; any other
            // RDF name breaks section 7.
            _ => return Ok(Opened::Broken),
        };
        let attributes_allowed =
            child
                .attributes()
                .all(|attribute| match (attribute.namespace, attribute.local) {
                    (None, _) => true,
                    (Some(XML), local) => local != "lang",
                    (Some(_), _) => false,
                });
        if !attributes_allowed {
            return Ok(Opened::Broken);
        }
        Ok(Opened::Frame(Frame::items(element, child.children(), kind)))
    }

    /// The structure an `rdf:Description` inside a property element holds:
    /// its attributes are fields, and its child elements the others
    /// (sections 7.6 and 7.9.2.2).
    fn description_value(
        &mut self,
        element: Element<'d>,
        description: Element<'d>,
    ) -> Result<Opened<'d>> {
        let mut frame = Frame::fields(element, description.children());
        for attribute in description.attributes() {
            match (attribute.namespace, attribute.local) {
                (Some(RDF), "type") => {}
                // rdf:about, rdf:ID and rdf:nodeID have no place in a
                // value (section 7.4).
                (Some(RDF), _) | (Some(XML), "lang") => return Ok(Opened::Broken),
                (None | Some(XML), _) => continue,
                (Some(_), _) => {}
            }
            let value = self.count(XmpValue::simple(attribute.value), description.offset())?;
            frame.add(
                attribute.namespace.unwrap_or_default(),
                attribute.local,
                value,
            );
        }
        Ok(Opened::Frame(frame))
    }
}

/// What [`Walk::open`] found in a property element.
enum Opened<'d> {
    /// A value read whole.
    Value(XmpValue),
    /// A structure or an array, whose elements the walk reads next.
    Frame(Frame<'d>),
    /// RDF that XMP Part 1 forbids: the property is left out.
    Broken,
}

/// A structure or an array still being read.
struct Frame<'d> {
    /// The property, field or item element whose value this is.
    element: Element<'d>,
    /// The elements still to read: the fields, or the `rdf:li` items.
    children: Children<'d>,
    kind: FrameKind<'d>,
}

enum FrameKind<'d> {
    Fields {
        fields: Vec<XmpProperty>,
        /// The name of each field, to leave a repeat out.
        names: HashSet<(&'d str, &'d str)>,
    },
    Items {
        kind: XmpArrayKind,
        items: Vec<XmpValue>,
    },
}

impl<'d> Frame<'d> {
    fn fields(element: Element<'d>, children: Children<'d>) -> Self {
        Self {
            element,
            children,
            kind: FrameKind::Fields {
                fields: Vec::new(),
                names: HashSet::new(),
            },
        }
    }

    fn items(element: Element<'d>, children: Children<'d>, kind: XmpArrayKind) -> Self {
        Self {
            element,
            children,
            kind: FrameKind::Items {
                kind,
                items: Vec::new(),
            },
        }
    }

    /// Whether `child` may stand in this structure or array: a field whose
    /// name XMP allows, or an `rdf:li` item.
    fn allows(&self, child: Element<'_>) -> bool {
        match self.kind {
            FrameKind::Fields { .. } => property_namespace(child).is_some(),
            FrameKind::Items { .. } => child.is(RDF, "li"),
        }
    }

    /// Takes the value of `child`, one of this frame's elements; whether
    /// the RDF allows it.
    fn take(&mut self, child: Element<'d>, value: XmpValue) -> bool {
        match &mut self.kind {
            FrameKind::Fields { .. } => {
                self.add(child.namespace().unwrap_or_default(), child.local(), value);
            }
            FrameKind::Items { items, .. } => items.push(value),
        }
        true
    }

    /// Adds a field, unless the structure gave its name before.
    fn add(&mut self, namespace: &'d str, local: &'d str, value: XmpValue) {
        if let FrameKind::Fields { fields, names } = &mut self.kind
            && names.insert((namespace, local))
        {
            fields.push(XmpProperty::new(namespace, local, value));
        }
    }

    /// The structure or array, read whole.
    fn finish(self) -> XmpValue {
        XmpValue::of(match self.kind {
            FrameKind::Fields { fields, .. } => XmpForm::Structure(fields),
            FrameKind::Items { kind, items } => XmpForm::Array { kind, items },
        })
    }
}

/// The value of an attribute that stands for a property, a field or a
/// qualifier: a simple value, but a URI for `rdf:type`, whose value RDF
/// reads as a URI reference, as a typed node's type is.
fn attribute_value(attribute: Attribute<'_>) -> XmpValue {
    match (attribute.namespace, attribute.local) {
        (Some(RDF), "type") => XmpValue::uri(attribute.value),
        _ => XmpValue::simple(attribute.value),
    }
}

/// Whether a top-level `rdf:Description` holds a property set XMP Part 1
/// allows (§7.4): no text but white space, no `xml:lang`, and no RDF
/// attribute but `rdf:about` and `rdf:type`.
fn is_property_set(description: Element<'_>) -> bool {
    let text = description.children().any(|child| match child {
        Child::Text(run) => !xml::is_blank(run),
        Child::Element(_) => false,
    });
    !text
        && description
            .attributes()
            .all(|attribute| match (attribute.namespace, attribute.local) {
                (Some(RDF), local) => local == "about" || local == "type",
                (Some(XML), local) => local != "lang",
                _ => true,
            })
}

/// The namespace of a property element, if XMP allows its name for a
/// property (XMP Part 1, §6.2). Of the RDF namespace, only `rdf:type` is
/// allowed; nothing of the XML namespace, and nothing without one.
fn property_namespace(element: Element<'_>) -> Option<&str> {
    match (element.namespace(), element.local()) {
        (Some(RDF), local) if local != "type" => None,
        (Some(XML) | None, _) => None,
        (namespace, _) => namespace,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use crate::namespaces::{DC, XMP};
    use crate::xmp::XmpForm;

    /// The `xmlns` declarations a packet's tests use.
    const NS: &str = "xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#' \
        xmlns:xmp='http://ns.adobe.com/xap/1.0/' xmlns:dc='http://purl.org/dc/elements/1.1/' \
        xmlns:ex='urn:example:'";

    /// A packet of `descriptions` in `rdf:RDF` in `x:xmpmeta`.
    fn packet(descriptions: &str) -> String {
        format!(
            "<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='test'><rdf:RDF {NS}>{descriptions}</rdf:RDF></x:xmpmeta>"
        )
    }

    /// The properties of one `rdf:Description` holding `body`.
    fn read_body(body: &str) -> Xmp {
        let text = packet(&format!(
            "<rdf:Description rdf:about=''>{body}</rdf:Description>"
        ));
        Xmp::from_bytes(text.as_bytes()).expect("a packet")
    }

    fn text<'a>(xmp: &'a Xmp, namespace: &str, name: &str) -> Option<&'a str> {
        xmp.property(namespace, name).and_then(XmpValue::text)
    }

    fn names(xmp: &Xmp) -> Vec<&str> {
        xmp.iter().map(XmpProperty::name).collect()
    }

    #[test]
    fn a_packet_in_xmpmeta_gives_its_toolkit_and_properties() {
        let xmp = read_body("<xmp:Rating>3</xmp:Rating>");
        assert_eq!(xmp.toolkit(), Some("test"));
        assert_eq!(xmp.about(), None);
        assert_eq!(text(&xmp, XMP, "Rating"), Some("3"));
        assert_eq!(xmp.len(), 1);
    }

    #[test]
    fn a_bare_rdf_element_is_a_packet_with_no_toolkit() {
        let text = format!("<rdf:RDF {NS}><rdf:Description xmp:Rating='2'/></rdf:RDF>");
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(xmp.toolkit(), None);
        assert_eq!(names(&xmp), ["Rating"]);
    }

    #[test]
    fn the_first_rdf_element_is_read_wherever_it_lies() {
        // XMP Part 1, section 7.3.1, allows other XML around rdf:RDF.
        let text = format!(
            "<doc xmlns:x='adobe:ns:meta/'><other/><x:xmpmeta x:xmptk='inner'><wrap><rdf:RDF {NS}>\
             <rdf:Description xmp:Rating='4'/></rdf:RDF></wrap></x:xmpmeta>\
             <rdf:RDF {NS}><rdf:Description xmp:Label='second'/></rdf:RDF></doc>"
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(xmp.toolkit(), Some("inner"));
        assert_eq!(names(&xmp), ["Rating"]);
    }

    #[test]
    fn xml_with_no_rdf_element_is_damage() {
        let err = Xmp::from_bytes(b"<x:xmpmeta xmlns:x='adobe:ns:meta/'/>").expect_err("damage");
        assert_eq!(err.kind(), ErrorKind::Malformed);
        assert_eq!(
            err.to_string(),
            "the XML holds no rdf:RDF element at byte 0"
        );
    }

    #[test]
    fn an_empty_rdf_element_is_an_empty_packet() {
        // As the D600 and other Nikon bodies write.
        let xmp = Xmp::from_bytes(packet("").as_bytes()).expect("a packet");
        assert!(xmp.is_empty());
        assert_eq!(xmp.toolkit(), Some("test"));
        let empty = Xmp::from_bytes(packet("<rdf:Description rdf:about=''/>").as_bytes());
        assert!(empty.expect("a packet").is_empty());
    }

    #[test]
    fn a_property_as_an_attribute_and_as_an_element_gives_a_simple_value() {
        let text = packet(
            "<rdf:Description rdf:about='' xmp:Rating='3'><xmp:Label>Red</xmp:Label></rdf:Description>",
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(names(&xmp), ["Rating", "Label"]);
        assert_eq!(xmp.property(XMP, "Rating"), Some(&XmpValue::simple("3")));
        assert_eq!(xmp.property(XMP, "Label"), Some(&XmpValue::simple("Red")));
    }

    #[test]
    fn a_simple_value_keeps_its_white_space_and_may_be_empty() {
        let xmp = read_body("<dc:format>  image/jpeg\n</dc:format><xmp:Label/><ex:a></ex:a>");
        assert_eq!(text(&xmp, DC, "format"), Some("  image/jpeg\n"));
        assert_eq!(text(&xmp, XMP, "Label"), Some(""));
        assert_eq!(text(&xmp, "urn:example:", "a"), Some(""));
    }

    #[test]
    fn an_rdf_resource_gives_a_uri() {
        let xmp = read_body("<xmp:BaseURL rdf:resource='https://example.com/'/>");
        assert_eq!(
            xmp.property(XMP, "BaseURL").map(XmpValue::form),
            Some(&XmpForm::Uri("https://example.com/".to_owned()))
        );
        // Text beside the resource breaks section 7.5.
        let xmp = read_body(
            "<xmp:BaseURL rdf:resource='https://example.com/'>text</xmp:BaseURL><xmp:Label>kept</xmp:Label>",
        );
        assert_eq!(names(&xmp), ["Label"]);
    }

    #[test]
    fn several_descriptions_are_one_property_set_in_document_order() {
        let text = packet(
            "<rdf:Description rdf:about='' xmp:Rating='1'/>\
             <rdf:Description rdf:about=''><dc:format>image/jpeg</dc:format></rdf:Description>\
             <rdf:Description rdf:about='' xmp:Label='Red'/>",
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(names(&xmp), ["Rating", "format", "Label"]);
    }

    #[test]
    fn a_repeated_property_gives_its_first_value() {
        let xmp = read_body("<xmp:Label>first</xmp:Label><xmp:Label>second</xmp:Label>");
        assert_eq!(text(&xmp, XMP, "Label"), Some("first"));
        assert_eq!(xmp.len(), 1);
        // As an attribute, then an element: the attribute comes first.
        let text = packet(
            "<rdf:Description rdf:about='' xmp:Rating='3'><xmp:Rating>5</xmp:Rating></rdf:Description>\
             <rdf:Description rdf:about='' xmp:Rating='4'/>",
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(xmp.property(XMP, "Rating"), Some(&XmpValue::simple("3")));
        assert_eq!(xmp.len(), 1);
    }

    #[test]
    fn the_xmp_namespace_under_any_prefix_is_found_by_one_lookup() {
        // The Nikon 1 bodies bind the XMP namespace to `xap`.
        let xap = "<x:xmpmeta xmlns:x='adobe:ns:meta/'><rdf:RDF \
            xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>\
            <rdf:Description rdf:about='' xmlns:xap='http://ns.adobe.com/xap/1.0/' xap:Rating='0'/>\
            </rdf:RDF></x:xmpmeta>";
        let xmp = Xmp::from_bytes(xap.as_bytes()).expect("a packet");
        assert_eq!(text(&xmp, XMP, "Rating"), Some("0"));
    }

    #[test]
    fn about_gives_the_first_resource_that_is_not_empty() {
        let text = packet(
            "<rdf:Description rdf:about=''/><rdf:Description xmp:Rating='1'/>\
             <rdf:Description rdf:about='uuid:1'/><rdf:Description rdf:about='uuid:2'/>",
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(xmp.about(), Some("uuid:1"));
    }

    #[test]
    fn a_name_xmp_forbids_for_a_property_costs_that_property_alone() {
        let xmp = read_body(
            "<rdf:li>no array</rdf:li><rdf:value>v</rdf:value><xml:space>s</xml:space>\
             <plain>no namespace</plain><rdf:type rdf:resource='urn:example:Kind'/>\
             <xmp:Label>kept</xmp:Label>",
        );
        assert_eq!(names(&xmp), ["type", "Label"]);
        assert_eq!(text(&xmp, RDF, "type"), Some("urn:example:Kind"));
    }

    #[test]
    fn a_description_that_breaks_the_rules_costs_its_own_properties() {
        let text = packet(
            "<rdf:Description rdf:about='' xmp:Rating='1'>stray text</rdf:Description>\
             <rdf:Description rdf:ID='d' xmp:Label='a'/>\
             <rdf:Description rdf:nodeID='n' xmp:Label='b'/>\
             <rdf:Description xml:lang='en' xmp:Label='c'/>\
             <ex:typed xmp:Label='d'/>\
             <rdf:Description rdf:about='uuid:ignored' rdf:ID='x'/>\
             <rdf:Description rdf:about=''><dc:format>kept</dc:format></rdf:Description>",
        );
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(names(&xmp), ["format"]);
        assert_eq!(xmp.about(), None);
    }

    #[test]
    fn an_rdf_type_attribute_is_a_uri() {
        let text = packet("<rdf:Description rdf:type='urn:example:Kind' xmp:Rating='1'/>");
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(
            xmp.property(RDF, "type"),
            Some(&XmpValue::uri("urn:example:Kind"))
        );
        assert_eq!(xmp.property(XMP, "Rating"), Some(&XmpValue::simple("1")));
    }

    #[test]
    fn attributes_with_no_namespace_or_of_the_xml_namespace_are_ignored() {
        let text = packet("<rdf:Description about='' xml:space='preserve' xmp:Rating='1'/>");
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("a packet");
        assert_eq!(names(&xmp), ["Rating"]);
    }

    #[test]
    fn values_are_read_to_the_limit_and_no_further() {
        let text = packet(
            "<rdf:Description rdf:about='' xmp:Rating='1' xmp:Label='Red'><dc:format>f</dc:format></rdf:Description>",
        );
        let at_limit = Limits::default().with_max_xmp_values(3);
        let xmp = Xmp::read(text.as_bytes(), &at_limit).expect("three values");
        assert_eq!(xmp.len(), 3);
        let past = Limits::default().with_max_xmp_values(2);
        let err = Xmp::read(text.as_bytes(), &past).expect_err("past the limit");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
        assert!(
            err.to_string()
                .starts_with("the XMP packet gives more values than the limit")
        );
        // A repeated property counts, though it is left out.
        let repeated = packet("<rdf:Description xmp:Rating='1'/><rdf:Description xmp:Rating='2'/>");
        let one = Limits::default().with_max_xmp_values(1);
        assert!(Xmp::read(repeated.as_bytes(), &one).is_err());
    }

    /// The structure `{stDim:w: 8.5, stDim:h: 11.0}`, in the example
    /// namespace.
    fn page_size() -> XmpValue {
        XmpValue::structure([
            XmpProperty::new("urn:example:", "w", "8.5"),
            XmpProperty::new("urn:example:", "h", "11.0"),
        ])
    }

    #[test]
    fn a_structure_in_each_of_its_three_forms_gives_one_value() {
        let xmp = read_body(
            "<ex:a rdf:parseType='Resource'><ex:w>8.5</ex:w><ex:h>11.0</ex:h></ex:a>\
             <ex:b><rdf:Description><ex:w>8.5</ex:w><ex:h>11.0</ex:h></rdf:Description></ex:b>\
             <ex:c ex:w='8.5' ex:h='11.0'/>\
             <ex:d><rdf:Description ex:w='8.5'><ex:h>11.0</ex:h></rdf:Description></ex:d>",
        );
        assert_eq!(names(&xmp), ["a", "b", "c", "d"]);
        for property in &xmp {
            assert_eq!(property.value(), &page_size(), "{}", property.name());
        }
    }

    #[test]
    fn an_empty_structure_is_a_structure() {
        let xmp = read_body("<ex:a rdf:parseType='Resource'/><ex:b><rdf:Description/></ex:b>");
        for property in &xmp {
            assert_eq!(property.value().form(), &XmpForm::Structure(Vec::new()));
        }
        assert_eq!(xmp.len(), 2);
    }

    #[test]
    fn bag_seq_and_alt_give_the_three_kinds_of_array() {
        let xmp = read_body(
            "<dc:subject><rdf:Bag><rdf:li>harbor</rdf:li><rdf:li>sunrise</rdf:li></rdf:Bag></dc:subject>\
             <dc:creator><rdf:Seq><rdf:li>Claude Monet</rdf:li></rdf:Seq></dc:creator>\
             <dc:rights><rdf:Alt><rdf:li>Public domain</rdf:li></rdf:Alt></dc:rights>",
        );
        let kinds = [
            (
                "subject",
                XmpArrayKind::Unordered,
                vec!["harbor", "sunrise"],
            ),
            ("creator", XmpArrayKind::Ordered, vec!["Claude Monet"]),
            ("rights", XmpArrayKind::Alternative, vec!["Public domain"]),
        ];
        for (name, kind, items) in kinds {
            let value = xmp.property(DC, name).expect(name);
            let expected = XmpValue::array(kind, items.into_iter().map(XmpValue::from));
            assert_eq!(value, &expected, "{name}");
        }
    }

    #[test]
    fn an_empty_bag_is_an_empty_array() {
        // As editors write dc:subject with no keyword.
        let xmp = read_body("<dc:subject><rdf:Bag/></dc:subject>");
        let subject = xmp.property(DC, "subject").expect("subject");
        assert_eq!(subject, &XmpValue::array(XmpArrayKind::Unordered, []));
        assert!(subject.items().is_empty());
    }

    #[test]
    fn values_nest_in_any_form() {
        let xmp = read_body(
            "<ex:list><rdf:Seq>\
               <rdf:li rdf:parseType='Resource'><ex:w>8.5</ex:w><ex:h>11.0</ex:h></rdf:li>\
               <rdf:li ex:w='8.5' ex:h='11.0'/>\
               <rdf:li rdf:resource='urn:example:item'/>\
               <rdf:li><rdf:Bag><rdf:li>inner</rdf:li></rdf:Bag></rdf:li>\
             </rdf:Seq></ex:list>\
             <ex:outer rdf:parseType='Resource'><ex:tags><rdf:Bag><rdf:li>t</rdf:li></rdf:Bag></ex:tags>\
               <ex:inner rdf:parseType='Resource'><ex:leaf>deep</ex:leaf></ex:inner></ex:outer>",
        );
        let expected = XmpValue::array(
            XmpArrayKind::Ordered,
            [
                page_size(),
                page_size(),
                XmpValue::uri("urn:example:item"),
                XmpValue::array(XmpArrayKind::Unordered, ["inner".into()]),
            ],
        );
        assert_eq!(xmp.property("urn:example:", "list"), Some(&expected));
        let outer = xmp.property("urn:example:", "outer").expect("outer");
        let tags = outer.field("urn:example:", "tags").expect("tags");
        assert_eq!(tags.items(), [XmpValue::simple("t")]);
        let leaf = outer
            .field("urn:example:", "inner")
            .and_then(|inner| inner.field("urn:example:", "leaf"));
        assert_eq!(leaf.and_then(XmpValue::text), Some("deep"));
    }

    #[test]
    fn a_repeated_field_gives_its_first_value() {
        let xmp = read_body(
            "<ex:a rdf:parseType='Resource'><ex:w>first</ex:w><ex:w>second</ex:w></ex:a>\
             <ex:b><rdf:Description ex:w='attribute'><ex:w>element</ex:w></rdf:Description></ex:b>",
        );
        let a = xmp.property("urn:example:", "a").expect("a");
        assert_eq!(a.fields().len(), 1);
        assert_eq!(
            a.field("urn:example:", "w").and_then(XmpValue::text),
            Some("first")
        );
        let b = xmp.property("urn:example:", "b").expect("b");
        assert_eq!(
            b.field("urn:example:", "w").and_then(XmpValue::text),
            Some("attribute")
        );
    }

    #[test]
    fn each_form_xmp_forbids_costs_its_property_and_no_other() {
        for (form, broken) in [
            (
                "parseType Literal",
                "<ex:p rdf:parseType='Literal'><b>x</b></ex:p>",
            ),
            (
                "parseType Collection",
                "<ex:p rdf:parseType='Collection'><ex:i/></ex:p>",
            ),
            ("rdf:ID", "<ex:p rdf:ID='id'>x</ex:p>"),
            ("rdf:nodeID", "<ex:p rdf:nodeID='n'/>"),
            ("rdf:datatype", "<ex:p rdf:datatype='urn:t'>1</ex:p>"),
            (
                "rdf:li outside an array",
                "<ex:p rdf:parseType='Resource'><rdf:li>x</rdf:li></ex:p>",
            ),
            ("text beside an element", "<ex:p>text<rdf:Bag/></ex:p>"),
            (
                "text in a structure",
                "<ex:p rdf:parseType='Resource'>text<ex:f>x</ex:f></ex:p>",
            ),
            (
                "text in an array",
                "<ex:p><rdf:Bag>text<rdf:li>x</rdf:li></rdf:Bag></ex:p>",
            ),
            ("two elements", "<ex:p><rdf:Bag/><rdf:Bag/></ex:p>"),
            (
                "rdf:about inside",
                "<ex:p><rdf:Description rdf:about=''><ex:f>x</ex:f></rdf:Description></ex:p>",
            ),
            (
                "xml:lang on rdf:Description",
                "<ex:p><rdf:Description xml:lang='en'><ex:f>x</ex:f></rdf:Description></ex:p>",
            ),
            (
                "fields as attributes and elements",
                "<ex:p ex:f='1'><ex:g>2</ex:g></ex:p>",
            ),
            (
                "fields as attributes with parseType",
                "<ex:p rdf:parseType='Resource' ex:f='1'/>",
            ),
            (
                "fields as attributes with text",
                "<ex:p ex:f='1'>text</ex:p>",
            ),
            (
                "a resource with fields",
                "<ex:p rdf:resource='urn:x' ex:f='1'/>",
            ),
            (
                "a resource with parseType",
                "<ex:p rdf:resource='urn:x' rdf:parseType='Resource'/>",
            ),
            (
                "an array item not rdf:li",
                "<ex:p><rdf:Bag><ex:i>x</ex:i></rdf:Bag></ex:p>",
            ),
            (
                "rdf:_1",
                "<ex:p><rdf:Seq><rdf:_1>x</rdf:_1></rdf:Seq></ex:p>",
            ),
            (
                "an attribute on an array",
                "<ex:p><rdf:Bag ex:a='1'><rdf:li>x</rdf:li></rdf:Bag></ex:p>",
            ),
            (
                "xml:lang on an array",
                "<ex:p><rdf:Alt xml:lang='en'><rdf:li>x</rdf:li></rdf:Alt></ex:p>",
            ),
            ("an RDF element in a value", "<ex:p><rdf:Statement/></ex:p>"),
            (
                "a field with no namespace",
                "<ex:p rdf:parseType='Resource'><f>x</f></ex:p>",
            ),
            (
                "a field in the XML namespace",
                "<ex:p rdf:parseType='Resource'><xml:f>x</xml:f></ex:p>",
            ),
            (
                "damage deep inside",
                "<ex:p rdf:parseType='Resource'><ex:q><rdf:Bag><rdf:li rdf:ID='i'/></rdf:Bag></ex:q></ex:p>",
            ),
        ] {
            let xmp = read_body(&format!(
                "<xmp:Label>before</xmp:Label>{broken}<xmp:Rating>3</xmp:Rating>"
            ));
            assert_eq!(names(&xmp), ["Label", "Rating"], "{form}");
        }
    }

    #[test]
    fn white_space_and_comments_between_elements_are_no_text() {
        let xmp = read_body(
            "<dc:subject>\n  <!-- keywords -->\n  <rdf:Bag>\n    <rdf:li>a</rdf:li>\n  </rdf:Bag>\n</dc:subject>",
        );
        let subject = xmp.property(DC, "subject").expect("subject");
        assert_eq!(subject.items(), [XmpValue::simple("a")]);
    }

    #[test]
    fn nested_values_count_toward_the_limit() {
        // The property, its two fields, the second an array, and that
        // array's two items: 5 values.
        let body = "<ex:p rdf:parseType='Resource'><ex:a>1</ex:a>\
            <ex:b><rdf:Bag><rdf:li>x</rdf:li><rdf:li>y</rdf:li></rdf:Bag></ex:b></ex:p>";
        let text = packet(&format!("<rdf:Description>{body}</rdf:Description>"));
        let five = Limits::default().with_max_xmp_values(5);
        let xmp = Xmp::read(text.as_bytes(), &five).expect("five values");
        assert_eq!(xmp.len(), 1);
        let four = Limits::default().with_max_xmp_values(4);
        let err = Xmp::read(text.as_bytes(), &four).expect_err("past the limit");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
        // Fields as attributes count too: the structure and its two fields.
        let attributes = packet("<rdf:Description><ex:p ex:a='1' ex:b='2'/></rdf:Description>");
        let two = Limits::default().with_max_xmp_values(2);
        assert!(Xmp::read(attributes.as_bytes(), &two).is_err());
        let three = Limits::default().with_max_xmp_values(3);
        assert!(Xmp::read(attributes.as_bytes(), &three).is_ok());
    }

    #[test]
    fn values_nest_as_deep_as_the_xml_limit_allows() {
        // x:xmpmeta, rdf:RDF and rdf:Description, then 61 property levels.
        let levels = 61;
        let body = format!(
            "{}x{}",
            "<ex:p rdf:parseType='Resource'>".repeat(levels - 1) + "<ex:p>",
            "</ex:p>".repeat(levels)
        );
        let text = packet(&format!("<rdf:Description>{body}</rdf:Description>"));
        let xmp = Xmp::from_bytes(text.as_bytes()).expect("64 levels");
        let mut value = xmp.property("urn:example:", "p");
        for _ in 1..levels {
            value = value.and_then(|value| value.field("urn:example:", "p"));
        }
        assert_eq!(value.and_then(XmpValue::text), Some("x"));
        let deeper = text.replacen("<ex:p>", "<ex:p rdf:parseType='Resource'><ex:p>", 1);
        let err = Xmp::from_bytes(deeper.as_bytes()).expect_err("65 levels");
        assert_eq!(err.kind(), ErrorKind::LimitExceeded);
    }

    #[test]
    fn damaged_xml_costs_the_packet() {
        let err =
            Xmp::from_bytes(packet("<rdf:Description><xmp:Label>x</rdf:Description>").as_bytes())
                .expect_err("damage");
        assert_eq!(err.kind(), ErrorKind::Malformed);
    }
}
