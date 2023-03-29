use sqlx::PgPool;
use shuttle_secrets::SecretStore;
use axum_extra::extract::cookie::Key;
use axum::extract::FromRef;
use dotenvy::dotenv;

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

#[tokio::main]
async fn main()  {

    dotenv.ok();
    
    let postgres = dotenvy::var("DATABASE_URL").expect("No database URL was set!");

    let postgres = sqlx::Pool::connect(&postgres).await.unwrap();

    sqlx::migrate!()
        .run(&postgres)
        .await
        .expect("Migrations failed :(");

    let stripe_token = dotenvy.var("STRIPE_API_KEY").expect("Couldn't find STRIPE_API_KEY!");

    let smtp_email = dotenvy.var("SMTP_EMAIL").expect("Couldn't find SMTP_EMAIL!");

    let smtp_password = dotenvy.var("SMTP_PASSWORD").expect("Couldn't find SMTP_PASSWORD!");

    let state = AppState {postgres, stripe_token, smtp_email, smtp_password, key: Key::generate()};
    
    let router = create_router(state);

    axum::Server::bind(0.0.0.0:8000).serve(router.into_make_service()).await
}