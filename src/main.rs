use fast_kv::core::processor::RespCommandProcessor;
use fast_kv::server::server::Server;

fn main() {
    let _log_guard = fast_kv::logger::init_logger();
    println!("Starting fast_kv server");
    Server::new(RespCommandProcessor::new()).run().unwrap();
    println!("fast_kv server stopped");
}
