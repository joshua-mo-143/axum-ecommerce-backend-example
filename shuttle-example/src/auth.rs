
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json
};
use axum_extra::extract::cookie::{Cookie, PrivateCookieJar, SameSite};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize};
use sqlx::Row;
use time::Duration;

use crate::AppState;

#[derive(Deserialize)]
pub struct RegisterDetails {
    username: String,
    email: String,
    password: String
}

#[derive(Deserialize)]
pub struct LoginDetails {
    username: String,
    password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(newuser): Json<RegisterDetails>,
) -> impl IntoResponse {
    let hashed_password = bcrypt::hash(newuser.password, 10).unwrap();

    let query = sqlx::query("INSERT INTO users (username, email, password) values ($1, $2, $3)")
        .bind(newuser.username)
        .bind(newuser.email)
        .bind(hashed_password)
        .execute(&state.postgres);

    match query.await {
        Ok(_) => (StatusCode::CREATED, "Account created!".to_string()).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            format!("Something went wrong: {e}"),
        )
            .into_response(),
    }
}

pub async fn login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    Json(login): Json<LoginDetails>,
) -> Result<(PrivateCookieJar, StatusCode), StatusCode> {
    let query = sqlx::query("SELECT * FROM users WHERE username = $1")
        .bind(&login.username)
        .fetch_one(&state.postgres);

    match query.await {
        Ok(res) => {
            
            if bcrypt::verify(login.password, res.get("password")).is_err() {
                return Err(StatusCode::BAD_REQUEST);
            }
            let session_id = rand::random::<u64>().to_string();

            sqlx::query("INSERT INTO sessions (session_id, user_id) VALUES ($1, $2) ON CONFLICT (user_id) DO UPDATE SET session_id = EXCLUDED.session_id")
                .bind(&session_id)
                .bind(res.get::<i32, _>("id"))
                .execute(&state.postgres)
                .await
                .expect("Couldn't insert session :(");

            let cookie = Cookie::build("foo", session_id)
                .secure(true)
                .same_site(SameSite::Strict)
                .http_only(true)
                .path("/")
                .max_age(Duration::WEEK)
                .finish();

            Ok((jar.add(cookie), StatusCode::OK))
        }

        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

pub async fn logout(State(state): State<AppState>, jar: PrivateCookieJar) -> Result<PrivateCookieJar, StatusCode> {
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {
        return Ok(jar)
    };

    let query = sqlx::query("DELETE FROM sessions WHERE session_id = $1")
        .bind(cookie)
        .execute(&state.postgres);


        match query.await {
        Ok(_) => Ok(jar.remove(Cookie::named("foo"))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR)        
    }
}

pub async fn forgot_password(
    State(state): State<AppState>,
    Json(email_recipient): Json<String>,
) -> Response {
    let new_password = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);

    let hashed_password = bcrypt::hash(&new_password, 10).unwrap();

    sqlx::query("UPDATE users SET password = $1 WHERE email = $2")
            .bind(hashed_password)
            .bind(email_recipient.clone())
            .execute(&state.postgres)
            .await.expect("Had an error resetting password");

    let credentials = Credentials::new(state.smtp_email, state.smtp_password);

    let message = format!("Hello!\n\n Your new password is: {new_password} \n\n Don't share this with anyone else. \n\n Kind regards, \nZest");

    let email = Message::builder()
        .from("noreply <joshua.mo.876@gmail.com".parse().unwrap())
        .to(email_recipient.parse().unwrap())
        .subject("Forgot Password")
        .header(ContentType::TEXT_PLAIN)
        .body(message)
        .unwrap();

    let mailer = SmtpTransport::relay("smtp.gmail.com")
        .unwrap()
        .credentials(credentials)
        .build();

    match mailer.send(&email) {
        Ok(_) => (StatusCode::OK, "Sent".to_string()).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, format!("Error: {e}")).into_response(),
    }
}

pub async fn validate_session<B>(
    jar: PrivateCookieJar,
    State(state): State<AppState>,
    request: Request<B>,
    next: Next<B>,
) -> (PrivateCookieJar, Response) {
    let Some(cookie) = jar.get("foo").map(|cookie| cookie.value().to_owned()) else {

        println!("Couldn't find a cookie in the jar");
        return (jar,(StatusCode::FORBIDDEN, "Forbidden!".to_string()).into_response())
    };

    let find_session = sqlx::query("SELECT * FROM sessions WHERE session_id = $1")
        .bind(cookie)
        .execute(&state.postgres)
        .await;

    match find_session {
        Ok(_) => (jar, next.run(request).await),
        Err(_) => (
            jar,
            (StatusCode::FORBIDDEN, "Forbidden!".to_string()).into_response(),
        ),
    }
}
