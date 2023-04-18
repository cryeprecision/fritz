use roxmltree::Node;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("field `{0}` not found")]
    FieldNotFound(&'static str),
    #[error("field `{0}` is not a text node")]
    NoText(&'static str),
}
pub type Result<T> = std::result::Result<T, Error>;

pub fn find_node_by_tag<'a, 'b>(node: Node<'a, 'b>, tag: &'static str) -> Result<Node<'a, 'b>> {
    node.children()
        .find(|n| n.has_tag_name(tag))
        .ok_or(Error::FieldNotFound(tag))
}

pub fn find_text_by_tag<'a>(node: Node<'a, '_>, tag: &'static str) -> Result<&'a str> {
    match find_node_by_tag(node, tag) {
        Ok(node) => node.text().ok_or(Error::NoText(tag)),
        Err(err) => Err(err),
    }
}
