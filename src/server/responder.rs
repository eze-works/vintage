use super::{files, router, ServerSpec};
use crate::connection::Connection;
use crate::context::{Request, Response};
use crate::error::Error;
use crate::record::*;
use crate::status;
use convert_case::{Case, Casing};
use std::collections::BTreeMap;

// Handles a FastCGI Connection.
//
// There are two expected flows;
// + We receive a `GetValues` request to which we respond.
// + We receive a `BeginRequest` request followed by Params and Stdin. Respond using Stdout followed by EndRequest
pub fn handle_connection(mut conn: Connection, spec: ServerSpec) {
    let first_record = match conn.read_record() {
        Ok(r) => r,
        Err(e) => {
            return handle_error(&mut conn, e);
        }
    };

    if let Record::GetValues(r) = first_record {
        return handle_get_values(&mut conn, r);
    }

    let Record::BeginRequest(begin) = first_record else {
        log::error!("FastCGI connection began with unexpected record. Closing connection");
        return;
    };

    if begin.keep_alive() {
        let response =
            Record::EndRequest(EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported));
        let _ = conn.write_record(&response);
        log::warn!("FastCGI client wanted keep-alive. It is not supported. Closing connection");
        return;
    }

    let mut params = match conn.expect_params() {
        Ok(params) => params,
        Err(None) => {
            log::error!("FastCGI connection missing Params record. Closing connection");
            return;
        }
        Err(Some(e)) => {
            return handle_error(&mut conn, e);
        }
    };

    let mut stdin = match conn.expect_stdin() {
        Ok(stdin) => stdin,
        Err(None) => {
            log::error!("FastCGI connection missing Stdin record. Closing connection");
            return;
        }
        Err(Some(e)) => {
            return handle_error(&mut conn, e);
        }
    };

    let mut vars = params.take();

    let Some(method) = vars.remove("REQUEST_METHOD") else {
        log::error!("FastCGI request missing REQUEST_METHOD header. Closing connection.");
        return;
    };

    let Some(path) = vars.remove("PATH_INFO") else {
        log::error!("FastCGI request missing PATH_INFO header. Closing connection.");
        return;
    };

    let Some(query_string) = vars.remove("QUERY_STRING") else {
        log::error!("FastCGI request missing QUERY_STRING header. Closing connection.");
        return;
    };

    let mut headers = BTreeMap::new();
    for (k, v) in vars {
        if let Some(suffix) = k.strip_prefix("HTTP_") {
            headers.insert(suffix.to_case(Case::Train), v);
        }
    }

    let mut request = Request::default();
    request.method = method;
    request.path = path;
    request.query_string = query_string;
    request.headers = headers;
    request.body = stdin.take();

    let mut response: Option<Response> = None;

    if let Some(fs_spec) = spec.file_server {
        if let Some(suffix) = request.path.strip_prefix(&fs_spec.request_prefix) {
            response = Some(files::respond(suffix, &fs_spec.fs_path, &request));
        }
    };

    if response.is_none() {
        if let Some(router_spec) = spec.router {
            response = router::respond(&mut request, router_spec);
        }
    }

    if response.is_none() {
        if let Some(fallback) = spec.fallback {
            response = Some(fallback(&mut request));
        }
    }

    let response = response.unwrap_or(Response::default().set_status(status::NOT_FOUND));

    let mut stdout = Stdout(vec![]);
    let _ = response.write_stdout_bytes(&mut stdout.0);
    let _ = conn.write_record(&Record::Stdout(stdout));

    let _ = conn.write_record(&Record::EndRequest(EndRequest::new(
        0,
        ProtocolStatus::RequestComplete,
    )));
}

fn handle_error(conn: &mut Connection, e: Error) {
    match e {
        Error::UnsupportedRole(_) => {
            let response = EndRequest::new(0, ProtocolStatus::UnknownRole);
            let _ = conn.write_record(&response.into());
            log::warn!("FastCGI client requested an unknown role. Closing connection");
        }
        Error::MultiplexingUnsupported => {
            let response = EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported);
            let _ = conn.write_record(&response.into());
            log::warn!("FastCGI client requested connection multiplixing. It is not supported. Closing connection");
        }
        Error::UnknownRecordType(t) => {
            let response = UnknownType(t);
            let _ = conn.write_record(&response.into());
            log::warn!("Unknown record type: {t}. Closing connection");
        }
        e => {
            log::warn!(error:err = e; "Error reading FastCGI record. Closing connection");
        }
    }
}

fn handle_get_values(conn: &mut Connection, record: GetValues) {
    let mut response = GetValuesResult::default();
    for variable in record.get_variables() {
        // If the client cares, tell it we do not want to multiplex connections
        if variable == "FCGI_MPXS_CONNS" {
            response = response.add("FCGI_MPXS_CONNS", "0");
            break;
        }
    }
    let _ = conn.write_record(&Record::GetValuesResult(response));
}
