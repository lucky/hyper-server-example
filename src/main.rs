use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use errors::Errors;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio_postgres::{Config, NoTls};

mod errors;
mod tasks;

lazy_static! {
    static ref COMPLETE_TASK_RE: regex::Regex =
        regex::Regex::new(r"^/tasks/(\d+)/complete$").expect("regex creation failed");
}

fn make_response(status: StatusCode, body: Body) -> Response<Body> {
    Response::builder()
        .status(status)
        .body(body)
        .expect("Failed to create response")
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

async fn post_task(
    task_dao: Arc<tasks::TaskDAO>,
    req: Request<Body>,
) -> Result<Response<Body>, Errors> {
    let content_type = match req.headers().get("Content-Type") {
        Some(ct) => ct.to_str().expect("Failed to parse content type"),
        None => "",
    };
    if !content_type.contains("application/json") {
        return Ok(do400("Content-Type must be application/json".into()));
    }
    let whole_body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(Errors::Hyper)?;
    let t: tasks::Task = serde_json::from_slice(&whole_body.slice(0..)).map_err(Errors::Json)?;
    task_dao.insert(t).await?;
    Ok(do200("".into()))
}

async fn get_tasks(task_dao: Arc<tasks::TaskDAO>) -> Result<Response<Body>, Errors> {
    let tasks = task_dao.select_all().await?;
    let json = serde_json::to_string(&tasks).map_err(Errors::Json)?;
    Ok(Response::new(Body::from(json)))
}

async fn complete_task(id: i32, task_dao: Arc<tasks::TaskDAO>) -> Result<Response<Body>, Errors> {
    let task = task_dao.get(id).await?;
    match task {
        Some(_) => {
            if task_dao.mark_complete(id).await? {
                Ok(do200("".into()))
            } else {
                Ok(do400("Task is already complete".into()))
            }
        }
        None => Ok(do404()),
    }
}

async fn serve(
    task_dao: Arc<tasks::TaskDAO>,
    req: Request<Body>,
) -> Result<Response<Body>, hyper::Error> {
    let result = match (req.method(), req.uri().path()) {
        (&Method::POST, "/tasks") => post_task(task_dao, req).await,
        (&Method::GET, "/tasks") => get_tasks(task_dao).await,
        (&Method::POST, path) if COMPLETE_TASK_RE.is_match(path) => {
            let captures = COMPLETE_TASK_RE.captures(path).expect("regex failed");
            if let Ok(id) = captures[1].parse::<i32>() {
                complete_task(id, task_dao).await
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
        Err(Errors::Pool(e)) => {
            eprintln!("Pool error: {}", e);
            Ok(do500())
        }
    }
}

async fn create_table(pool: &Pool) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = pool.get().await?;
    client
        .execute(
            "CREATE TABLE IF NOT EXISTS tasks (
        id SERIAL,
        person TEXT,
        description TEXT,
        created_at TIMESTAMP WITH TIME ZONE DEFAULT now(),
        completed_at TIMESTAMP WITH TIME ZONE NULL
      );",
            &[],
        )
        .await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3030).into();
    let config = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set")
        .parse::<Config>()?;
    let mgr_config = ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    };
    let mgr = Manager::from_config(config, NoTls, mgr_config);
    let pool = Pool::builder(mgr)
        .max_size(16)
        .build()
        .expect("Failed to create pool");
    create_table(&pool).await?;
    let task_dao = Arc::new(tasks::TaskDAO::new(pool));

    let make_service = make_service_fn(move |_| {
        let task_dao = task_dao.clone();

        async move {
            Ok::<_, Error>(service_fn(move |req| {
                let task_dao = task_dao.clone();
                async move { serve(task_dao, req).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
