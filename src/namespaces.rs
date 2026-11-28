// SPDX-License-Identifier: MIT OR Apache-2.0

//! The URIs of the XMP namespaces of the rating, the label, IPTC Core and
//! two qualifiers, so that a lookup says what it looks for.
//!
//! XMP names every property by a namespace URI and a local name, never by
//! the prefix a packet writes (XMP Part 1, §6.2). One packet may bind the
//! XMP namespace to `xmp` and another to `xap`; both mean
//! [`XMP`]. So [`Xmp::property`](crate::Xmp::property) takes the URI, and
//! each constant here is one such URI, named after the namespace:
//! `namespaces::DC` rather than `"http://purl.org/dc/elements/1.1/"`.
//!
//! There is a constant for each namespace that holds the rating, the
//! label or a property of IPTC Core, and for those of the qualifiers
//! `xml:lang` and `rdf:type`. Any other namespace a caller writes out:
//! Adobe Camera Raw's, a maker's or a phone's, the IPTC Extension's.
//! There are no constants for property names: a property is looked up by
//! the name its standard gives it, such as `"subject"` or `"Rating"`.
//!
//! # Examples
//!
//! Look up the keywords and the rating of a packet, by namespace and name:
//!
//! ```
//! use raw_files::{Xmp, XmpArrayKind, XmpValue, namespaces};
//!
//! // A packet as a caller's test builds it; a file's packet reads the same.
//! let xmp = Xmp::default()
//!     .with_property(namespaces::XMP, "Rating", "3")
//!     .with_property(
//!         namespaces::DC,
//!         "subject",
//!         XmpValue::array(XmpArrayKind::Unordered, ["harbor".into(), "sunrise".into()]),
//!     );
//! let rating = xmp.property(namespaces::XMP, "Rating").and_then(XmpValue::text);
//! assert_eq!(rating, Some("3"));
//! let keywords: Vec<&str> = xmp
//!     .property(namespaces::DC, "subject")
//!     .map(|subject| subject.items().iter().filter_map(XmpValue::text).collect())
//!     .unwrap_or_default();
//! assert_eq!(keywords, ["harbor", "sunrise"]);
//! ```

/// The XMP namespace, of the basic properties such as `Rating`, `Label`,
/// `CreateDate` and `CreatorTool` (XMP Part 1, §8.4). Packets bind it to
/// the prefix `xmp`, or `xap` on older writers.
pub const XMP: &str = "http://ns.adobe.com/xap/1.0/";

/// The Dublin Core namespace, of properties such as `creator`, `rights`,
/// `description`, `title` and `subject`, the keywords (XMP Part 1, §8.3).
/// Packets bind it to the prefix `dc`.
pub const DC: &str = "http://purl.org/dc/elements/1.1/";

/// The XMP Rights Management namespace, of properties such as `Marked`
/// and `WebStatement` (XMP Part 1, §8.5). Packets bind it to the prefix
/// `xmpRights`.
pub const XMP_RIGHTS: &str = "http://ns.adobe.com/xap/1.0/rights/";

/// The Photoshop namespace, of properties such as `DateCreated`, `City`,
/// `Credit` and `Headline` (XMP Part 2, §3.2). Packets bind it to the
/// prefix `photoshop`.
pub const PHOTOSHOP: &str = "http://ns.adobe.com/photoshop/1.0/";

/// The IPTC Core namespace, of properties such as `Location`,
/// `CountryCode` and the creator's contact information (IPTC Photo
/// Metadata Standard 2025.1, §6). IPTC Core's other properties lie in the
/// Dublin Core, Photoshop and XMP Rights namespaces. Packets bind it to
/// the prefix `Iptc4xmpCore`.
pub const IPTC_CORE: &str = "http://iptc.org/std/Iptc4xmpCore/1.0/xmlns/";

/// The XML namespace, of the qualifier `lang`, a value's language
/// (Namespaces in XML 1.0, §3). Packets write it as `xml:lang`, a prefix
/// that needs no declaration. [`XmpValue::language`](crate::XmpValue::language)
/// looks it up.
pub const XML: &str = "http://www.w3.org/XML/1998/namespace";

/// The RDF namespace, of the qualifier `type`, the kind of resource a
/// value describes (XMP Part 1, §6.2 and §7.9.2.5). Packets bind it to
/// the prefix `rdf`.
pub const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_namespace_ends_in_a_character_no_local_name_holds() {
        // XMP Part 1, §6.2, advises a slash or a number sign at the end, so
        // that a URI and a local name joined do not run together.
        for uri in [XMP, DC, XMP_RIGHTS, PHOTOSHOP, IPTC_CORE, RDF] {
            assert!(uri.ends_with('/') || uri.ends_with('#'), "{uri}");
        }
    }

    #[test]
    fn the_namespaces_are_distinct() {
        let all = [XMP, DC, XMP_RIGHTS, PHOTOSHOP, IPTC_CORE, XML, RDF];
        for (i, a) in all.iter().enumerate() {
            for b in &all[i + 1..] {
                assert_ne!(a, b);
            }
        }
    }
}
