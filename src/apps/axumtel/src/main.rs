use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use opentel::init_trace;
use opentelemetry::global;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;

mod opentel;

#[tokio::main]
#[tracing::instrument]
async fn main() -> Result<()> {
    // initialize tracing
    // tracing_subscriber::fmt::init();

    let tracer = init_trace()?;
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = tracing_subscriber::Registry::default().with(telemetry);
    tracing::subscriber::set_global_default(subscriber)?;

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    // tracer.in_current_span();
    global::shutdown_tracer_provider();

    Ok(())
}

// basic handler that responds with a static string
#[tracing::instrument]
async fn root() -> &'static str {
    info!("Calling root");
    "Hello, World!"
}

#[tracing::instrument]
async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    info!("Creating user with username: {}", payload.username);
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Debug, Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}