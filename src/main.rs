use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use errors::Errors;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};
use lazy_static::lazy_static;
use std::sync::Arc;
use tokio_postgres::{Config, NoTls};
use validation::TryIntoValid;

use crate::errors::UserErrors;

mod errors;
mod tasks;
mod validation;

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
    let content_type: InputContentTypes = req.headers().get("Content-Type").into();
    let whole_body = hyper::body::to_bytes(req.into_body())
        .await
        .map_err(Errors::Hyper)?;

    let t: tasks::TaskInput = match content_type {
        InputContentTypes::Json => {
            serde_json::from_slice(&whole_body.slice(0..)).map_err(Errors::Json)?
        }
        _ => return Ok(do400("Unknown content type".into())),
    };
    task_dao.insert(t.try_into_valid()?).await?;
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

async fn index(
    jinja: minijinja::Environment<'static>,
    task_dao: Arc<tasks::TaskDAO>,
    _req: Request<Body>,
) -> Result<Response<Body>, Errors> {
    let template = jinja.get_template("index.html").map_err(Errors::Template)?;
    let tasks = task_dao.select_all().await?;
    let body = template
        .render(minijinja::context! { tasks => tasks })
        .map_err(Errors::Template)?;
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/html")
        .body(body.into())
        .expect("Failed to create response"))
}

async fn serve(
    task_dao: Arc<tasks::TaskDAO>,
    req: Request<Body>,
    jinja: minijinja::Environment<'static>,
) -> Result<Response<Body>, hyper::Error> {
    let result = match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => index(jinja, task_dao, req).await,
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
        Err(Errors::Template(e)) => {
            eprintln!("Template error: {}", e);
            Ok(do500())
        }
        Err(Errors::Validation(e)) => {
            eprintln!("Validation error: {}", e);
            match serde_json::to_string::<UserErrors>(&e.into()) {
                Ok(json) => Ok(do400(json.into())),
                Err(e) => {
                    eprintln!("JSON error: {}", e);
                    Ok(do500())
                }
            }
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

enum InputContentTypes {
    Json,
    Form,
    Unknown,
}

impl From<Option<&hyper::header::HeaderValue>> for InputContentTypes {
    fn from(value: Option<&hyper::header::HeaderValue>) -> Self {
        match value {
            Some(v) => match v.to_str() {
                Ok(s) if s.starts_with("application/json") => InputContentTypes::Json,
                Ok(s) if s.starts_with("application/x-www-form-urlencoded") => {
                    InputContentTypes::Form
                }
                _ => InputContentTypes::Unknown,
            },
            None => InputContentTypes::Unknown,
        }
    }
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
    let mut jinja = minijinja::Environment::new();
    jinja.add_template("layout.html", include_str!("../templates/layout.html"))?;
    jinja.add_template("index.html", include_str!("../templates/index.html"))?;

    let make_service = make_service_fn(move |_| {
        let task_dao = task_dao.clone();
        let jinja = jinja.clone();
        async move {
            Ok::<_, Error>(service_fn(move |req| {
                let task_dao = task_dao.clone();
                let jinja = jinja.clone();
                async move { serve(task_dao, req, jinja).await }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_service);
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
