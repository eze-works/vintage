//! HTTP status code constants

macro_rules! status_codes {
    ($($name:ident  $value:literal),* $(,)?) => {
        $(
            pub const $name: u16 = $value;
        )*
    }
}

status_codes! {
    OK                          200,
    NOT_MODIFIED                304,
    TEMPORARY_REDIRECT          307,
    PERMANENT_REDIRECT          308,
    BAD_REQUEST                 400,
    NOT_FOUND                   404,
    METHOD_NOT_ALLOWED          405,
    TEAPOT                      418,
    INTERNAL_SERVER_ERROR       500,
}
