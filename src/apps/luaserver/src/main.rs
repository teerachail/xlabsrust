use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use mlua::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    sync::{Arc, RwLock},
};
use surrealdb::engine::local::{Db, Mem};
use surrealdb::Surreal;
// use std::{collections::HashMap, fs};
use serde_json::Value as JsonValue;
use tokio::signal;

#[derive(Clone)]
struct AppManager<'a> {
    lua_man: Arc<RwLock<LuaStateManager<'a>>>,
    db: Arc<Surreal<Db>>,
}

impl<'a> AppManager<'a> {
    async fn new() -> Self {
        let db = Surreal::new::<Mem>(()).await.unwrap();
        let _ = db.use_ns("test").use_db("test").await.unwrap();
        let db = Arc::new(db);
        AppManager {
            lua_man: Arc::new(RwLock::new(LuaStateManager::new())),
            db,
        }
    }

    fn call_lua(&mut self, payload: JsonValue) -> String {
        let ro = self.lua_man.read().unwrap().clone();
        if ro.loaded {
            let luafn: LuaFunction = ro
                .luaval
                .as_table()
                .unwrap()
                .get::<_, LuaFunction>("test")
                .unwrap();
            let result: String = luafn.call(()).unwrap();
            result
        } else {
            let lua = Lua::new();
            let testluacontent = fs::read_to_string("test.lua").unwrap();
            let mut ro = self.lua_man.write().unwrap().clone();
            let result: LuaValue = lua
                .load(testluacontent.as_str())
                .eval::<LuaValue>()
                .unwrap();

            let luafn: LuaFunction = result
                .as_table()
                .unwrap()
                .get::<_, LuaFunction>("test")
                .unwrap();
            ro.luaval = result.clone();
            // ro.loaded = true; // This is not working, has no effect!!!
            let p = lua.to_value(&payload).unwrap();
            let pv = p.as_table().unwrap();
            pv.set("extra", "This is set from server!").unwrap();
            let result: String = luafn.call(p).unwrap();
            result
        }
    }

    async fn list(&self) -> Result<Vec<JsonValue>> {
        // Ok(vec![])
        let people = self.db.select("person").await?;
        Ok(people)
    }

    async fn save(&self, person: JsonValue) -> Result<Vec<JsonValue>> {
        let created = self.db.create("person").content(person).await?;
        Ok(created)
    }
}

struct LuaStateManager<'a> {
    luaval: LuaValue<'a>,
    loaded: bool,
}

impl<'a> Clone for LuaStateManager<'a> {
    fn clone(&self) -> Self {
        LuaStateManager {
            luaval: LuaValue::Nil,
            loaded: false,
        }
    }
}

impl<'a> LuaStateManager<'a> {
    fn new() -> Self {
        LuaStateManager {
            luaval: LuaValue::Nil,
            loaded: false,
        }
    }

    fn free(&mut self) {
        // self.lua.drop();
    }
}

impl<'a> Drop for LuaStateManager<'a> {
    fn drop(&mut self) {
        self.free();
    }
}

// Implement Send and Sync manually (requires reasoning about thread safety)
unsafe impl<'a> Send for LuaStateManager<'a> {}
unsafe impl<'a> Sync for LuaStateManager<'a> {}

#[tokio::main]
async fn main() -> Result<()> {
    let appman = AppManager::new().await;
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/test", post(testapi))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .route("/people", get(list_people).post(create_person))
        .with_state(appman);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Help reduce error message from db
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

// basic handler that responds with a static string
async fn testapi<'a>(
    State(mut appman): State<AppManager<'a>>,
    Json(payload): Json<JsonValue>,
) -> (StatusCode, String) {
    let fval = appman.call_lua(payload);
    // let msg = "Hello, Test !!!".to_string();
    let msg = format!("Hello, Test !!! {}", fval);
    (StatusCode::OK, msg)
}

// basic handler that responds with a static string
async fn root() -> (StatusCode, String) {
    (StatusCode::OK, "Hello, World!".to_string())
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

async fn create_person<'a>(
    State(appman): State<AppManager<'a>>,
    Json(payload): Json<JsonValue>,
) -> (StatusCode, Json<Vec<JsonValue>>) {
    let people = appman.save(payload).await.unwrap();
    (StatusCode::OK, Json(people))
}

async fn list_people<'a>(
    State(appman): State<AppManager<'a>>,
) -> (StatusCode, Json<Vec<JsonValue>>) {
    let people = appman.list().await.unwrap();
    (StatusCode::OK, Json(people))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
