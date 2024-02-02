use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, Response, StatusCode},
    middleware::{self, Next},
    routing::{get, post},
    Json, Router,
};
use tokio::sync::mpsc::{channel, Sender};

type MyState = (Arc<AtomicUsize>, Arc<AtomicBool>, Sender<()>);

// hit with `curl 0.0.0.0:3000 -X POST -v`
#[tokio::main]
async fn main() {
    let counter = Arc::new(AtomicUsize::default());
    let should_append_header = Arc::new(AtomicBool::default());
    let (send_shutdown, recv_shutdown) = channel::<()>(1);
    let app = Router::new()
        .route("/", post(handle_post))
        .route("/", get(handle_get))
        .with_state((counter, should_append_header.clone(), send_shutdown))
        .layer(middleware::from_fn_with_state(
            should_append_header,
            close_conn_header,
        ));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(recv_shutdown))
        .await
        .unwrap();
}

async fn handle_get(State((counter, _, _)): State<MyState>) -> Json<usize> {
    Json(counter.load(Ordering::Acquire))
}

async fn handle_post(
    State((counter, should_append_header, shutdown)): State<MyState>,
) -> Response<Body> {
    println!("post received");
    let val = counter.fetch_add(1, Ordering::AcqRel);
    println!("Counter is {val}");
    if val > 5 {
        println!("Starting to close connections");
        should_append_header.store(true, Ordering::Release);
    }
    if val == 10 {
        let _ = shutdown.send(()).await;
    }
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("hi\n"))
        .unwrap()
}

async fn shutdown_signal(mut recv: tokio::sync::mpsc::Receiver<()>) {
    recv.recv().await;
}

async fn close_conn_header(
    State(should_append_header): State<Arc<AtomicBool>>,
    request: Request,
    next: Next,
) -> Response<Body> {
    let mut response = next.run(request).await;

    if should_append_header.load(Ordering::Acquire) {
        response
            .headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    response
}
