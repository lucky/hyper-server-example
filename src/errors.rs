use serde::Serialize;

#[derive(Debug)]
pub enum Errors {
    Database(tokio_postgres::Error),
    Json(serde_json::Error),
    Hyper(hyper::Error),
    Pool(deadpool_postgres::PoolError),
    Validation(validator::ValidationErrors),
}

#[derive(Serialize)]
pub struct UserErrors {
    pub errors: Vec<UserError>,
}

#[derive(Serialize)]
pub struct UserError {
    pub field: String,
    pub errors: Vec<validator::ValidationError>,
}

pub fn map_errors(e: validator::ValidationErrors) -> UserErrors {
    let mut errors = Vec::new();
    for (field, validation_errors) in e.field_errors() {
        let mut field_errors = Vec::new();
        for error in validation_errors {
            field_errors.push(error.clone());
        }
        errors.push(UserError {
            field: field.to_string(),
            errors: field_errors,
        });
    }
    UserErrors { errors }
}
