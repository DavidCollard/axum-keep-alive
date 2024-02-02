use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    extract::State,
    http::{header, Response, StatusCode},
    routing::post,
    Router,
};

// hit with `curl 0.0.0.0:3000 -X POST -v`
#[tokio::main]
async fn main() {
    let counter = Arc::new(AtomicUsize::default());
    let (send, recv) = tokio::sync::mpsc::channel::<()>(1);
    let app = Router::new()
        .route("/", post(handle_post))
        .with_state((counter, send));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(recv))
        .await
        .unwrap();
}

async fn handle_post(
    State((counter, send)): State<(Arc<AtomicUsize>, tokio::sync::mpsc::Sender<()>)>,
) -> Response<Body> {
    println!("post received");
    let val = counter.fetch_add(1, Ordering::AcqRel);
    println!("Counter is {val}");

    let mut res = Response::builder().status(StatusCode::OK);
    if val % 3 == 0 {
        res = res.header(header::CONNECTION, "close");
    }
    if val == 10 {
        let _ = send.send(()).await;
    }
    res.body(Body::from("hi\n")).unwrap()
}

async fn shutdown_signal(mut recv: tokio::sync::mpsc::Receiver<()>) {
    recv.recv().await;
}
