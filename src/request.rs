use crate::record::{Params, Stdin};

/// A request from a FastCGI client. 
///
/// Common [request metavariables](https://datatracker.ietf.org/doc/html/rfc3875#section-4.1) can be retrieved using the similarly named functions.
/// These functions return `None` when the header has no value or if its value is the empty string.
///
/// Note that the presence/absence of these metavariables is dependent on the FastCGI client.
/// The FastCGI client might also forward HTTP headers prefixed with `HTTP_*`.
/// Use [`Request::get`] to access such metavariables for which a dedicated method does not
/// exist.
#[derive(Debug, Clone)]
pub struct Request {
    pub(crate) vars: Params,
    pub(crate) body: Stdin,
}

impl Request {
    impl_meta![
        AUTH_TYPE :: "Returns the mechanism used by the server to authenticate the user, if any.",
        CONTENT_LENGTH :: "Returns the size of the message body attached to the request, if any.",
        CONTENT_TYPE :: "Returns the media type of the request body if it exists.",
        GATEWAY_INTERFACE :: "Returns the CGI version being used.",
        PATH_INFO :: "Returns the path requested.",
        QUERY_STRING :: "Returns the URL-encoded search or parameter string.",
        REMOTE_ADDR :: "Returns the address of the client sending the request.",
        REMOTE_HOST :: "Returns the fully qualified address of the client sending the request.",
        REMOTE_USER :: "Returns the user identification string supplied by the client as part of user authentication.",
        REQUEST_METHOD :: "Returns the request method used.",
        SERVER_NAME :: "Returns the name of the server to which this request was directed.",
        SERVER_PORT :: "Returns the TCP/IP port on which this request was received",
        SERVER_PROTOCOL :: "Returns the name and version of the application protocol used for this request",
        SERVER_SOFTWARE :: "Returns the name and version of the FastCGI client"
    ];

    /// Returns the value of the CGI meta-variable `name`, if it exists
    pub fn get(&self, name: &str) -> Option<&str> {
        let res = self.vars.get(name);
        if let Some("") = res {
            return None;
        }
        res
    }

    /// Returns the body of the request.
    ///
    /// The returned `Vec` will be empty if the request had no body.
    /// If there was a body, note that subsequent invocations will return an empty `Vec`.
    pub fn read_body(&mut self) -> Vec<u8> {
        self.body.take()
    }
}

macro_rules! impl_meta {
    ($($name:ident :: $doc:literal),*) => {
        $(
        paste::paste! {
            #[doc = $doc]
            pub fn [<get_$name:lower>](&self) -> Option<&str> {
                self.get(stringify!($name))
            }

        }
        )*
    };
}
pub(crate) use impl_meta;
