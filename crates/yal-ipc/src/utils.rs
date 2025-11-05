use std::path::PathBuf;

pub fn socket_path() -> PathBuf {
    "/tmp/yal_ipc.sock".into()
}

pub fn context() -> tarpc::context::Context {
    tarpc::context::current()
}

pub fn context_with_deadline(duration: std::time::Duration) -> tarpc::context::Context {
    let deadline = std::time::Instant::now() + duration;
    let mut context = tarpc::context::Context::current();
    context.deadline = deadline;
    context
}
