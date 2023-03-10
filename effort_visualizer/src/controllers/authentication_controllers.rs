use super::errors::ApiError;
use crate::repositories;
use crate::{domain::users::User, helpers::environments::EnvVariables};

use actix_session::Session;
use actix_web::{
    post,
    web::{self, Data},
    HttpResponse,
};
use anyhow::{Context, Result};

use repositories::users_repository::UserRepository;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct LoginInfo {
    credential: String,
}

#[derive(Serialize, ToSchema)]
pub struct LoginResult {
    situation: LoginSituation,
}

#[derive(Serialize, ToSchema)]
pub enum LoginSituation {
    Succeeded,
    NotRegistered,
}

fn verify_token(env_variables: &EnvVariables, credential: &str) -> Result<google_signin::IdInfo> {
    let mut client = google_signin::Client::new();
    client
        .audiences
        .push(env_variables.google_client_id.to_owned());
    client
        .verify(credential)
        .context("Token verification failed.")
}

async fn find_user(
    user_repository: &Data<Box<dyn UserRepository>>,
    email: &str,
) -> Result<Option<User>, ApiError> {
    user_repository.find(email).await.map_err(ApiError::from)
}

async fn add_user(
    user_repository: &Data<Box<dyn UserRepository>>,
    data: &User,
) -> Result<(), ApiError> {
    user_repository.add(data).await.map_err(ApiError::from)
}

#[utoipa::path(
    post,
    request_body = LoginInfo,
    responses(
        (status = 200, description = "Login user", body = LoginResult),
        (status = 401, description = "Login failed"),
        (status = 500, description = "Internal error")
    ),
)]
#[post("/login")]
pub async fn login(
    session: Session,
    env_variables: Data<EnvVariables>,
    user_repository: Data<Box<dyn UserRepository>>,
    credential_info: web::Json<LoginInfo>,
) -> Result<HttpResponse, actix_web::Error> {
    let id_token = match verify_token(&env_variables, &credential_info.credential) {
        Ok(id_token) => id_token,
        Err(e) => return Ok(HttpResponse::Unauthorized().body(e.to_string())),
    };
    let email = match id_token.email {
        Some(email) => email,
        None => return Ok(HttpResponse::Unauthorized().body("Email is empty.")),
    };
    let user = match find_user(&user_repository, &email).await? {
        Some(user) => user,
        None => {
            return Ok(HttpResponse::Ok().json(LoginResult {
                situation: LoginSituation::NotRegistered,
            }))
        }
    };
    session.insert("current_user", user)?;
    Ok(HttpResponse::Ok().json(LoginResult {
        situation: LoginSituation::Succeeded,
    }))
}
#[derive(Deserialize, ToSchema)]
pub struct SignupInfo {
    token: LoginInfo,
    user_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct SignupResult {
    situation: SignupSituation,
}

#[derive(Serialize, ToSchema)]
pub enum SignupSituation {
    Succeeded,
    AlreadyRegistered,
}

#[utoipa::path(
    post,
    request_body = SignupInfo,
    responses(
        (status = 200, description = "Sign up user", body = SignupResult),
        (status = 401, description = "Login failed"),
        (status = 500, description = "Internal error")
    ),
)]
#[post("/signup")]
pub async fn signup(
    session: Session,
    env_variables: Data<EnvVariables>,
    user_repository: Data<Box<dyn UserRepository>>,
    signup_info: web::Json<SignupInfo>,
) -> Result<HttpResponse, actix_web::Error> {
    if signup_info.user_name.is_empty() {
        return Ok(HttpResponse::BadRequest().body("Username is empty."));
    }

    let id_token = match verify_token(&env_variables, &signup_info.token.credential) {
        Ok(id_token) => id_token,
        Err(e) => return Ok(HttpResponse::Unauthorized().body(e.to_string())),
    };
    let email = match id_token.email {
        Some(email) => email,
        None => return Ok(HttpResponse::Unauthorized().body("Email is empty.")),
    };

    if find_user(&user_repository, &email).await?.is_some() {
        return Ok(HttpResponse::Ok().json(SignupResult {
            situation: SignupSituation::AlreadyRegistered,
        }));
    }

    let new_user = User {
        email,
        external_id: id_token.sub,
        user_name: signup_info.user_name.to_owned(),
        registered_date: std::time::SystemTime::now(),
        updated_date: std::time::SystemTime::now(),
    };
    add_user(&user_repository, &new_user).await?;
    session.insert("current_user", new_user)?;
    Ok(HttpResponse::Ok().json(SignupResult {
        situation: SignupSituation::Succeeded,
    }))
}
