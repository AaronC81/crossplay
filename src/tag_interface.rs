use anyhow::{Result, anyhow};
use id3::{frame::Comment, Tag, TagLike};

/// A custom item of metadata which is stored inside an MP3 file, as an ID3 comment.
/// 
/// This wrapper trait facilitates converting to/from the string content of the tag, and handling
/// the case where a tag is missing.
/// 
/// More precisely, the `CustomTag::NAME` field is used as the "text" of the comment, and the value
/// is the "description".
pub trait CustomTag {
    /// The type of value which this tag represents. Loading the tag returns this type by parsing
    /// the comment's text with `from_comment_text`, and saving converts it to a string using
    /// `to_comment_text`.
    type T;

    /// The full ID3 name of the comment.
    const NAME: &'static str;

    /// Converts the contents of the comment's text into this tag's value type.
    fn from_comment_text(str: &str) -> Self::T;

    /// Converts this tag's value into a string value for the comment.
    /// 
    /// If this returns `None`, the comment is explicitly deleted (or left uncreated).
    fn to_comment_text(value: Self::T) -> Option<String>;

    /// A default value to load if the tag is missing.
    /// 
    /// If this returns `None`, then `read_custom_tag` will return an error if the tag is missing.
    fn value_if_comment_missing() -> Option<Self::T>;
}

/// An extension trait implemented only on `id3::tag::Tag`.
pub trait CustomTagExtensions {
    /// Writes custom metadata as a comment into this tag, overwriting any previous value. Depending
    /// on the tag, this may also delete the comment entirely.
    fn write_custom<C: CustomTag>(&mut self, value: C::T);

    /// Reads custom metadata as a comment from this tag. If the comment is missing, then depending
    /// on the tag, this may either return a default value or an error.
    fn read_custom<C: CustomTag>(&self) -> Result<C::T>;
}

impl CustomTagExtensions for Tag {
    fn write_custom<C: CustomTag>(&mut self, value: C::T) {
        // Delete existing comment
        self.remove_comment(Some(C::NAME), None);

        if let Some(text) = C::to_comment_text(value) {
            // Write new comment
            self.add_frame(Comment {
                description: C::NAME.to_string(),
                text,
                lang: "eng".to_string(),
            });
        } else {
            // Leave the comment deleted
        }
    }

    fn read_custom<C: CustomTag>(&self) -> Result<C::T> {
        // Try to find matching comment
        if let Some(comment) = self.comments().find(|c| c.description == C::NAME) {
            // Nice, we found one! Convert to value
            Ok(C::from_comment_text(&comment.text))
        } else {
            // Missing - fall back to default value, if allowed
            if let Some(value) = C::value_if_comment_missing() {
                Ok(value)
            } else {
                Err(anyhow!("missing required metadata item: {}", C::NAME))
            }
        }
    }
}

/// A boolean metadata item, where the value is true if the comment is present, and false if the
/// comment is not present.
pub trait FlagTag {
    const NAME: &'static str;
}
impl<X: FlagTag> CustomTag for X {
    type T = bool;
    const NAME: &'static str = X::NAME;

    fn from_comment_text(_: &str) -> Self::T {
        // The presence of this comment means the flag is true
        true
    }
    fn to_comment_text(value: Self::T) -> Option<String> {
        if value {
            Some("".to_string())
        } else {
            None
        }
    }

    fn value_if_comment_missing() -> Option<Self::T> {
        // If the flag is missing, it's false
        Some(false)
    }
}

pub struct YouTubeIdTag;
impl CustomTag for YouTubeIdTag {
    type T = String;
    const NAME: &'static str = "[CrossPlay] YouTube ID";

    fn from_comment_text(str: &str) -> Self::T { str.to_string() }
    fn to_comment_text(value: Self::T) -> Option<String> { Some(value) }
    fn value_if_comment_missing() -> Option<Self::T> { None }
}

pub struct CroppedTag;
impl FlagTag for CroppedTag {
    const NAME: &'static str = "[CrossPlay] Cropped";
}

pub struct MetadataEditedTag;
impl FlagTag for MetadataEditedTag {
    const NAME: &'static str = "[CrossPlay] Metadata edited";
}

pub struct DownloadTimeTag;
impl CustomTag for DownloadTimeTag {
    type T = u64;
    const NAME: &'static str = "[CrossPlay] Download time";

    fn from_comment_text(str: &str) -> Self::T { str.parse().unwrap() }
    fn to_comment_text(value: Self::T) -> Option<String> { Some(value.to_string()) }
    fn value_if_comment_missing() -> Option<Self::T> { Some(0) }
}
