#![deny(warnings)]
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

mod tasks;

use tasks::{insert_task, select_all_tasks, Task};

fn do500() -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from("Internal Server Error"))
        .unwrap())
}

async fn post_task(
    client: Arc<Client>,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    // TODO require application/json content-type
    let whole_body = hyper::body::to_bytes(req.into_body()).await?;
    let t: Task = match serde_json::from_slice(&whole_body.slice(0..)) {
        Ok(t) => t,
        Err(e) => {
            println!("error happened: {}", e);
            return do500();
        }
    };
    match insert_task(client, t).await {
        Ok(()) => Ok(Response::new(Body::empty())),
        Err(e) => {
            println!("Error talking to database! {:?}", e);
            Ok(Response::builder().status(500).body(Body::empty()).unwrap())
        }
    }
}

async fn get_tasks(
    client: Arc<Client>,
    _req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let tasks = match select_all_tasks(client).await {
        Ok(tasks) => tasks,
        Err(e) => {
            println!("Error talking to database! {:?}", e);
            return do500();
        }
    };
    let json = match serde_json::to_string(&tasks) {
        Ok(json) => json,
        Err(e) => {
            println!("error happened: {}", e);
            return do500();
        }
    };
    Ok(Response::new(Body::from(json)))
}

async fn serve(client: Arc<Client>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/tasks") => post_task(client, req).await,
        (&Method::GET, "/tasks") => get_tasks(client, req).await,

        // Return the 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3030).into();
    let (client, connection) = tokio_postgres::connect(
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set").as_str(),
        NoTls,
    )
    .await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    let client = Arc::new(client);

    let make_service = make_service_fn(move |_| {
        let client = client.clone();

        async move {
            Ok::<_, Error>(service_fn(move |_req| {
                let client = client.clone();
                async move { serve(client, _req).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
