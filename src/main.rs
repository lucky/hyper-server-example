use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio_postgres::{Client, NoTls};

mod tasks;

use tasks::{insert, select_all, Task};

lazy_static! {
    static ref COMPLETE_TASK_RE: regex::Regex =
        regex::Regex::new(r"^/tasks/(\d+)/complete$").expect("regex creation failed");
}

fn make_response(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder().status(status).body(body).unwrap()
}

fn do400(reason: Body) -> Response<Body> {
    make_response(StatusCode::BAD_REQUEST, reason)
}

fn do500() -> Response<Body> {
    make_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".into(),
    )
}

fn do404() -> Response<Body> {
    make_response(StatusCode::NOT_FOUND, "Not found".into())
}

fn do200(body: Body) -> Response<Body> {
    make_response(StatusCode::OK, body)
}

#[derive(Debug)]
enum Errors {
    Database(tokio_postgres::Error),
    Json(serde_json::Error),
    Hyper(hyper::Error),
}

async fn post_task(client: Arc<Client>, req: Request<Body>) -> Result<Response<Body>, Errors> {
    let content_type = match req.headers().get("Content-Type") {
        Some(ct) => ct.to_str().unwrap(),
        None => "",
    };
    if !content_type.contains("application/json") {
        return Ok(do400("Content-Type must be application/json".into()));
    }
    let whole_body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(Errors::Hyper)?;
    let t: Task = serde_json::from_slice(&whole_body.slice(0..)).map_err(Errors::Json)?;
    insert(client, t).await.map_err(Errors::Database)?;
    Ok(do200("".into()))
}

async fn get_tasks(client: Arc<Client>) -> Result<Response<Body>, Errors> {
    let tasks = select_all(client).await.map_err(Errors::Database)?;
    let json = serde_json::to_string(&tasks).map_err(Errors::Json)?;
    Ok(Response::new(Body::from(json)))
}

async fn complete_task(id: i32, client: Arc<Client>) -> Result<Response<Body>, Errors> {
    let task = tasks::get(id, client.clone())
        .await
        .map_err(Errors::Database)?;
    match task {
        Some(_) => {
            if tasks::mark_complete(id, client.clone())
                .await
                .map_err(Errors::Database)?
            {
                Ok(do200("".into()))
            } else {
                Ok(do400("Task is already complete".into()))
            }
        }
        None => Ok(do404()),
    }
}

async fn serve(client: Arc<Client>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let result = match (req.method(), req.uri().path()) {
        (&Method::POST, "/tasks") => post_task(client, req).await,
        (&Method::GET, "/tasks") => get_tasks(client).await,
        (&Method::POST, path) if COMPLETE_TASK_RE.is_match(path) => {
            let captures = COMPLETE_TASK_RE.captures(path).expect("regex failed");
            if let Ok(id) = captures[1].parse::<i32>() {
                complete_task(id, client).await
            } else {
                Ok(do400("Invalid id".into()))
            }
        }
        _ => Ok(do404()),
    };

    match result {
        Ok(response) => Ok(response),
        Err(Errors::Hyper(e)) => Err(e),
        Err(Errors::Json(e)) => {
            eprintln!("JSON error: {}", e);
            Ok(do500())
        }
        Err(Errors::Database(e)) => {
            eprintln!("Database error: {}", e);
            Ok(do500())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3030).into();
    let (client, connection) = tokio_postgres::connect(
        std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set")
            .as_str(),
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
            Ok::<_, Error>(service_fn(move |req| {
                let client = client.clone();
                async move { serve(client, req).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
