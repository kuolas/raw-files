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
//! one of its child elements:
//!
//! - an element with text alone is a simple value (§7.5), the text as
//!   written, white space included;
//! - an element with an `rdf:resource` attribute and no content is a URI
//!   (§7.5).
//!
//! Damage here has a smaller reach than in the XML. A property whose RDF
//! breaks XMP Part 1, §7, is left out, and the others stand. So is a
//! property whose name is in the RDF namespace, which XMP forbids but for
//! `rdf:type`, or the XML namespace, or no namespace at all: an XMP name
//! needs a namespace (§6.2). An `rdf:Description` that breaks §7.4
//! itself, with text in it, an `xml:lang`, or an RDF attribute other than
//! `rdf:about` and `rdf:type`, is left out with every property it holds.
//! A property repeated in the packet is left out after its first. An
//! attribute with no namespace is not an XMP name, and is ignored, as an
//! attribute of the XML namespace other than `xml:lang` is.
//!
//! The walk is a loop, never recursive. It counts every value it builds,
//! and stops with [`ErrorKind::LimitExceeded`] past
//! [`Limits::max_xmp_values`].
//!
//! [`ErrorKind::LimitExceeded`]: crate::ErrorKind::LimitExceeded

use std::collections::HashSet;

use super::{Xmp, XmpProperty, XmpValue};
use crate::error::{Error, Result};
use crate::limits::Limits;
use crate::namespaces::{RDF, XML};
use crate::xml::{self, Attribute, Child, Document, Element};

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

    /// The value of a property element; `None` when it is left out.
    fn property(&mut self, element: Element<'d>) -> Result<Option<XmpValue>> {
        let mut resource = None;
        for attribute in element.attributes() {
            match (attribute.namespace, attribute.local) {
                (Some(RDF), "resource") => resource = Some(attribute.value),
                (None, _) => {}
                (Some(XML), local) if local != "lang" => {}
                _ => return Ok(None),
            }
        }
        let mut text = String::new();
        for child in element.children() {
            match child {
                Child::Element(_) => return Ok(None),
                Child::Text(run) => text.push_str(run),
            }
        }
        let value = match resource {
            Some(uri) if xml::is_blank(&text) => XmpValue::uri(uri),
            Some(_) => return Ok(None),
            None => XmpValue::simple(text),
        };
        self.count(value, element.offset()).map(Some)
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

    #[test]
    fn damaged_xml_costs_the_packet() {
        let err =
            Xmp::from_bytes(packet("<rdf:Description><xmp:Label>x</rdf:Description>").as_bytes())
                .expect_err("damage");
        assert_eq!(err.kind(), ErrorKind::Malformed);
    }
}
