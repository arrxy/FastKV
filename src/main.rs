mod config;
mod server;
fn main() {
    server::server::run_sync_tcp_server();
}
