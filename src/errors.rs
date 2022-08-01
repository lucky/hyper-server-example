use serde::Serialize;

#[derive(Debug)]
pub enum Errors {
    Database(tokio_postgres::Error),
    Json(serde_json::Error),
    Hyper(hyper::Error),
    Pool(deadpool_postgres::PoolError),
    Validation(validator::ValidationErrors),
    Template(minijinja::Error),
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

impl From<validator::ValidationErrors> for UserErrors {
    fn from(val: validator::ValidationErrors) -> Self {
        let mut errs = Vec::new();
        for (field, validation_errors) in val.field_errors() {
            let mut field_errors = Vec::new();
            for error in validation_errors {
                field_errors.push(error.clone());
            }
            errs.push(UserError {
                field: field.to_string(),
                errors: field_errors,
            });
        }
        UserErrors { errors: errs }
    }
}
