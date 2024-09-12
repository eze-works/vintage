# vintage

Let's take it back to the 1990s. This library implements a multi-threaded server that speaks the [FastCGI protocol](https://www.mit.edu/~yandros/doc/specs/fcgi-spec.html).

## Try it out!

Browsers don't speak FastCGI protocol.
Thankfully, most popular web servers do. 
So to test locally, we'll have to have one such web server running.
I'll be using Nginx & Caddy as examples.
Pick one.

1. Download either [Nginx](https://nginx.org/) or [Caddy](https://caddyserver.com/)
2. Configure the web server to reverse proxy fastcgi:
   1. If you picked caddy, stick this in a file in the current directory called `Caddyfile`:
      ```
       localhost {
         reverse_proxy localhost:8000 {
           transport fastcgi
         }
       }
      ```

   2. If you picked nginx, stick this in a file in the current directory called `nginx.conf`:
      ```
       events { }
       http {
         server {
           location / {
             fastcgi_pass localhost:8000;
           }
         }
       }
      ```
3. Run the web server
   - Caddy: `sudo caddy run --config Caddyfile`
   - Nginx: `sudo nginx -p $(pwd) -c nginx.conf`
4. Run `cargo new --bin app && cd app && cargo add vintage`, and stick this in `main.rs`:
   ```rust
   use vintage::{start, Response};

   fn main() {
       let server = start("localhost:8000", |_request| {
           Response::html("<h1>Hello World</h1>")
       }).unwrap();

       server.join();
   }
   ```
5. Run `cargo run`
6. Visit `http://localhost` on your browser!
  

## Why?

- I've been waiting for an excuse to use thread pools.
- The common setup of having an HTTP Web Server like nginx or caddy reverse-proxying to _another_ internal HTTP server feels a bit silly:
  Why are there two programs re-parsing HTTP?
- To rage against the rust async server monoculture and pave my own way ✊.


## Similar libraries

- [fastcgi](https://crates.io/crates/fastcgi)
- [tokio-fastcgi](https://crates.io/crates/tokio-fastcgi)
- [fastcgi-server](https://github.com/TheJokr/fastcgi-server)


[^1]: The only solid one I know of is [rouille](https://crates.io/crates/rouille) but that has not been updated in a while, and now has compilation warnings, which don't spark joy :/
      Yes, those compilation warnings are the only reason I sought an alternative.

