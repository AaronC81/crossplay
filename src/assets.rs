use iced::image::Handle;

#[derive(Debug)]
pub struct ImageData(&'static [u8]);

impl From<ImageData> for Handle {
    fn from(data: ImageData) -> Self {
        Handle::from_memory(data.0.to_vec())
    }
}

macro_rules! image {
    ($const:ident, $name:ident) => {
        pub const $const: ImageData = ImageData(include_bytes!(concat!("../assets/", stringify!($name), ".png")));
    };
}

image!(CROP, crop);
image!(CROP_DISABLED, crop_disabled);
image!(DELETE, delete);
image!(EDIT, edit);
image!(RESTORE, restore);
image!(RESTORE_DISABLED, restore_disabled);
image!(HIDDEN, hidden);
image!(NOT_HIDDEN, not_hidden);
