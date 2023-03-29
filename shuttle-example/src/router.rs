use axum::{routing::{post, get}, extract::{State, Path}, Router, Json};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use stripe::{
    Client, CreatePaymentLink, CreatePaymentLinkLineItems, CreatePrice, CreateProduct, Currency,
    IdOrCreate, PaymentLink, Price, Product as StripeProduct,
};

use crate::AppState;

use crate::auth::{register, login, logout, forgot_password, validate_session};

#[derive(sqlx::FromRow, Deserialize, Serialize)]
pub struct Product {
	id: i32,
	name: String,
	brand: String,
	category: String,
	price: String
}

pub fn create_router(state: AppState) -> Router {

	let auth_router = Router::new()
		.route("/register", post(register))
		.route("/login", post(login))
		.route("/logout", get(logout))
		.route("/forgot", post(forgot_password));

let products_router = 	Router::new()
		.route("/", get(get_items))
		.route("/:id", get(get_one_item));
	
	Router::new()
		.route("/payments", get(checkout))
		.nest("/products", products_router)
		.nest("/auth", auth_router)
		.with_state(state)
		
}

async fn get_items(State(state): State<AppState>) -> impl IntoResponse {
	let products: Vec<Product> = sqlx::query_as("SELECT * FROM PRODUCTS")
		.fetch_all(&state.postgres)
		.await.expect("Had some problem retrieving data :(");

	Json(products)
}

async fn get_one_item(State(state): State<AppState>, Path(id): Path<i32>) -> impl IntoResponse {
	let product: Product = sqlx::query_as("SELECT * FROM PRODUCTS WHERE id = $1")
					.bind(id)
					.fetch_one(&state.postgres)
					.await.expect("Had some problem retrieving data :(");

	Json(product)
}

async fn checkout(State(state): State<AppState>) -> impl IntoResponse {
    let client = Client::new(&state.stripe_token);

    let product = {
        let mut create_product = CreateProduct::new("T-Shirt");
        create_product.metadata =
            Some([("async-stripe".to_string(), "true".to_string())].iter().cloned().collect());
        StripeProduct::create(&client, create_product).await.unwrap()
    };

    // and add a price for it in USD
    let price = {
        let mut create_price = CreatePrice::new(Currency::USD);
        create_price.product = Some(IdOrCreate::Id(&product.id));
        create_price.metadata =
            Some([("async-stripe".to_string(), "true".to_string())].iter().cloned().collect());
        create_price.unit_amount = Some(1000);
        create_price.expand = &["product"];
        Price::create(&client, create_price).await.unwrap()
    };

    println!(
        "created a product {:?} at price {} {}",
        product.name.unwrap(),
        price.unit_amount.unwrap() / 100,
        price.currency.unwrap()
    );

    let payment_link = PaymentLink::create(
        &client,
        CreatePaymentLink::new(vec![CreatePaymentLinkLineItems {
            quantity: 3,
            price: price.id.to_string(),
            ..Default::default()
        }]),
    )
    .await
    .unwrap();

	payment_link.url
}