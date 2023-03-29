use sqlx::PgPool;
use shuttle_secrets::SecretStore;
use axum_extra::extract::cookie::Key;
use axum::extract::FromRef;

mod router;
use router::create_router;

mod auth;

#[derive(Clone)]
pub struct AppState {
    postgres: PgPool,
    stripe_token: String,
    smtp_email: String,
    smtp_password: String, 
    key: Key
}

impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.key.clone()
    }
}


#[shuttle_runtime::main]
async fn axum(
    #[shuttle_shared_db::Postgres] postgres: PgPool,
    #[shuttle_secrets::Secrets] secrets: SecretStore
) -> shuttle_axum::ShuttleAxum {

    let stripe_token = secrets.get("STRIPE_API_KEY").expect("Couldn't find STRIPE_API_KEY!");

    let smtp_email = secrets.get("SMTP_EMAIL").expect("Couldn't find SMTP_EMAIL!");

    let smtp_password = secrets.get("SMTP_PASSWORD").expect("Couldn't find SMTP_PASSWORD!");

    let state = AppState {postgres, stripe_token, smtp_email, smtp_password, key: Key::generate()};
    
    let router = create_router(state);

    Ok(router.into())
}