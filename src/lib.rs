// Implementation notes
//
// - Use Caddy & Nginx as model web-servers for figuring out what the Fast-CGI protocol should look
// like.
// - Only focus on the "Responder" role
//   - The purpose of the Authorizer & Filter roles are not completely clear. More importatntly,
//   they are not implemented by Caddy or Nginx.
//   - The protocol allows for a message when a role is not recognized. Use this.
// - No support for connection multiplexing, meaning a connection is not going to be re-used
// across http requests.
//   - The protcol allows for this to be signaled. So we are still compliant.
//   - Nginx & Caddy only support request id = 1. They close the connection after the response is
//   received.
// - Each connection gets handled in a thread from a "Connection" thread pool.
// - The connection reads the request in full before dispatching it to a "Worker" thread poool
// - The API should involved a single function call that returns an instance of the server that can
// be stopped

mod connection;
mod error;
pub mod record;
