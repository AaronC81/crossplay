use iced::{pure::{Element, widget::{Row, Column, Button}}, container};

pub(crate) trait ElementContainerExtensions<'a, Message> where Self: Sized {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self;

    fn push_if<T: Into<Element<'a, Message>>>(self, condition: bool, child_fn: impl FnOnce() -> T) -> Self {
        if condition {
            self.push(child_fn())
        } else {
            self
        }
    }

    fn push_if_let<T: Into<Element<'a, Message>>, O>(self, option: &Option<O>, child_fn: impl FnOnce(&O) -> T) -> Self {
        if let Some(o) = option.as_ref() {
            self.push(child_fn(o))
        } else {
            self
        }
    }
}

#[allow(clippy::only_used_in_recursion)]
impl<'a, Message> ElementContainerExtensions<'a, Message> for Row<'a, Message> {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self { self.push(child) }
}

#[allow(clippy::only_used_in_recursion)]
impl<'a, Message> ElementContainerExtensions<'a, Message> for Column<'a, Message> {
    fn push(self, child: impl Into<Element<'a, Message>>) -> Self { self.push(child) }
}

pub(crate) trait ButtonExtensions<'a, Message> where Self: Sized {
    fn on_press(self, msg: Message) -> Self;

    fn on_press_if(self, condition: bool, msg: Message) -> Self {
        if condition {
            self.on_press(msg)
        } else {
            self
        }
    }
}

impl<'a, Message> ButtonExtensions<'a, Message> for Button<'a, Message> {
    fn on_press(self, msg: Message) -> Self { self.on_press(msg) }
}

pub struct ContainerStyleSheet(pub container::Style);
impl container::StyleSheet for ContainerStyleSheet { fn style(&self) -> container::Style { self.0 } }
