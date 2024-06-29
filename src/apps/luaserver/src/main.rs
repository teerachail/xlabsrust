use std::{fs, sync::{Arc, RwLock}};

use anyhow::Result;
use axum::{
    extract::State, http::StatusCode, routing::{get, post}, Json, Router
};
use mlua::prelude::*;
use serde::{Deserialize, Serialize};
// use std::{collections::HashMap, fs};
use tokio::signal;


#[derive(Clone)]
struct AppManager<'a> {
    lua_man: Arc<RwLock<LuaStateManager<'a>>>,
}

impl<'a> AppManager<'a> {
    fn new() -> Self {
        AppManager {
            lua_man: Arc::new(RwLock::new(LuaStateManager::new())),
        }
    }

    fn call_lua(&mut self) -> f64 {
        let ro = self.lua_man.read().unwrap().clone();
        if ro.loaded {
            let luafn: LuaFunction = ro.luaval.as_table().unwrap().get::<_, LuaFunction>("test").unwrap();
            let result: f64 = luafn.call(()).unwrap();
            result
        } else {
            let lua = Lua::new();
            let testluacontent = fs::read_to_string("test.lua").unwrap();
            let mut ro = self.lua_man.write().unwrap().clone();
            let result: LuaValue = lua
                .load(testluacontent.as_str())
                .eval::<LuaValue>()
                .unwrap();

            let luafn: LuaFunction = result.as_table().unwrap().get::<_, LuaFunction>("test").unwrap();
            ro.luaval = result.clone();
            let result: f64 = luafn.call(()).unwrap();
            result
        }
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
    // initialize tracing
    tracing_subscriber::fmt::init();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        .route("/test", get(testapi))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user))
        .with_state(AppManager::new());

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

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
async fn testapi<'a>(State(mut appman): State<AppManager<'a>>) -> (StatusCode, String) {
    let fval = appman.call_lua();
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
