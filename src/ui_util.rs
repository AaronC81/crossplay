use iced::{Element, Row, Column, container};

pub(crate) trait ElementContainerExtensions<'a, Message> where Self: Sized {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self;

    fn push_if<T: Into<Element<'a, Message>>>(self, condition: bool, child_fn: impl FnOnce() -> T) -> Self {
        if condition {
            self.push(child_fn())
        } else {
            self
        }
    }
}

impl<'a, Message> ElementContainerExtensions<'a, Message> for Row<'a, Message> {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self { self.push(child) }
}

impl<'a, Message> ElementContainerExtensions<'a, Message> for Column<'a, Message> {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self { self.push(child) }
}

pub struct ContainerStyleSheet(pub container::Style);
impl container::StyleSheet for ContainerStyleSheet { fn style(&self) -> container::Style { self.0 } }
