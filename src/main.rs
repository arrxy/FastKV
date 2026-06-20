mod config;
mod server;
mod core;
fn main() {
    server::server::run_sync_tcp_server();
}
