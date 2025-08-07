#[ctor::ctor]
fn init() {
    std::env::set_var("RDKAFKA_LOG_LEVEL", "0");
}
