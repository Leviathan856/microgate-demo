use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use microgate::{RequestContext, Response, Router, Server};
use serde::{Deserialize, Serialize};
use sysinfo::{System, ProcessRefreshKind};

#[derive(Serialize, Deserialize)]
struct SensorData {
    id: String,
    value: f64,
}

#[derive(Serialize, Deserialize)]
struct ActuatorCmd {
    id: String,
    command: String, // "ON" or "OFF"
}

// Emulating MicroGate static storage.
// Because Rust requires shared states to be thread-safe for async handlers, 
// using static `OnceLock` or `lazy_static` is idiomatic.
static COMPILED_VERSION: std::sync::OnceLock<String> = std::sync::OnceLock::new();
static EMBEDDED_SYS_ID: &str = "ARM-CORTEX-M4-SIM";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize standard logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Initialize static storage
    COMPILED_VERSION.set(env!("CARGO_PKG_VERSION").to_string()).unwrap();

    // Server analytics & System info state
    let start_time = Instant::now();
    let api_calls = Arc::new(Mutex::new(0usize));
    let mut sys = System::new_all();
    sys.refresh_all();
    let pid = sysinfo::get_current_pid().unwrap();
    let sys_arc = Arc::new(Mutex::new(sys));

    // Setup in-memory datastores representing IoT sensor and actuator states
    let sensors_db: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new({
        let mut initial = HashMap::new();
        initial.insert("temperature".to_string(), 22.5);
        initial.insert("humidity".to_string(), 45.0);
        initial.insert("pressure".to_string(), 1012.0);
        initial
    }));

    let actuators_db: Arc<Mutex<HashMap<String, bool>>> = Arc::new(Mutex::new({
        let mut initial = HashMap::new();
        initial.insert("cooling_system".to_string(), false);
        initial.insert("main_door_lock".to_string(), true);
        initial
    }));

    let mut router = Router::new();

    // 1. Static file serving (HTML Dashboard)
    router = router.get("/", |_ctx: RequestContext| async move {
        microgate::fs::serve_file("./public", "/index.html").await
    });

    // 2. System Status Endpoint (Uptime, shared state counting, CPU/RAM usage)
    let metrics_counter = Arc::clone(&api_calls);
    let system_monitor = Arc::clone(&sys_arc);
    router = router.get("/api/system", move |_ctx| {
        let metrics = Arc::clone(&metrics_counter);
        let monitor = Arc::clone(&system_monitor);
        async move {
            *metrics.lock().unwrap() += 1;
            let uptime = start_time.elapsed().as_secs();
            let count = *metrics.lock().unwrap();
            
            let mut mem_usage = 0;
            let mut cpu_usage = 0.0;
            
            if let Ok(mut sys) = monitor.lock() {
                sys.refresh_process_specifics(pid, ProcessRefreshKind::everything());
                if let Some(process) = sys.process(pid) {
                    mem_usage = process.memory() / 1024; // KB
                    cpu_usage = process.cpu_usage();
                }
            }

            let version = COMPILED_VERSION.get().unwrap_or(&"unknown".to_string()).clone();

            let json = format!(
                r#"{{"uptime_seconds": {}, "total_framework_api_calls": {}, "memory_kb": {}, "cpu_percent": {:.2}, "version": "{}", "system_id": "{}"}}"#, 
                uptime, count, mem_usage, cpu_usage, version, EMBEDDED_SYS_ID
            );
            
            Response::new()
                .with_header("Content-Type", "application/json")
                .with_body(json)
        }
    });

    // 3. GET Sensors Endpoint 
    let db_s_get = Arc::clone(&sensors_db);
    let st_s_get = Arc::clone(&api_calls);
    router = router.get("/api/sensors", move |_ctx| {
        let db = Arc::clone(&db_s_get);
        let st = Arc::clone(&st_s_get);
        async move {
            *st.lock().unwrap() += 1;
            let data = db.lock().unwrap();
            let json_body = serde_json::to_string(&*data).unwrap_or_else(|_| "{}".to_string());
            Response::new()
                .with_header("Content-Type", "application/json")
                .with_body(json_body)
        }
    });

    // 4. POST Sensors Endpoint (Reads raw JSON bodies natively)
    let db_s_post = Arc::clone(&sensors_db);
    let st_s_post = Arc::clone(&api_calls);
    router = router.post("/api/sensors", move |ctx: RequestContext| {
        let db = Arc::clone(&db_s_post);
        let st = Arc::clone(&st_s_post);
        async move {
            *st.lock().unwrap() += 1;
            let body_str = ctx.req.body_as_str().unwrap_or_default();
            
            match serde_json::from_str::<SensorData>(body_str) {
                Ok(sensor) => {
                    let mut data = db.lock().unwrap();
                    data.insert(sensor.id.clone(), sensor.value);
                    log::info!("Updated sensor {} to {}", sensor.id, sensor.value);
                    Response::new()
                        .with_status(201, "Created")
                        .with_header("Content-Type", "application/json")
                        .with_body(r#"{"status": "success", "message": "Sensor data persisted"}"#)
                }
                Err(e) => {
                    log::warn!("Failed to parse POST body: {} - Error: {}", body_str, e);
                    Response::bad_request()
                }
            }
        }
    });

    // 5. DELETE Sensors Endpoint (Removing a resource)
    let db_s_del = Arc::clone(&sensors_db);
    router = router.delete("/api/sensors", move |ctx: RequestContext| {
        let db = Arc::clone(&db_s_del);
        async move {
            // we will expect a JSON body with just the ID, or read from URI ideally. 
            // In a simple architecture, we can read body: {"id": "temperature"}
            let body_str = ctx.req.body_as_str().unwrap_or_default();
            match serde_json::from_str::<serde_json::Value>(body_str) {
                Ok(val) => {
                    if let Some(id) = val.get("id").and_then(|v| v.as_str()) {
                        let mut data = db.lock().unwrap();
                        if data.remove(id).is_some() {
                            log::info!("Deleted sensor {}", id);
                            Response::new()
                                .with_status(200, "OK")
                                .with_header("Content-Type", "application/json")
                                .with_body(r#"{"status": "deleted"}"#)
                        } else {
                            Response::new().with_status(404, "Not Found").with_body("Sensor not found")
                        }
                    } else {
                        Response::bad_request()
                    }
                }
                Err(_) => Response::bad_request()
            }
        }
    });

    // 6. GET Actuators Endpoint 
    let db_a_get = Arc::clone(&actuators_db);
    router = router.get("/api/actuators", move |_ctx| {
        let db = Arc::clone(&db_a_get);
        async move {
            let data = db.lock().unwrap();
            let json_body = serde_json::to_string(&*data).unwrap_or_default();
            Response::new()
                .with_header("Content-Type", "application/json")
                .with_body(json_body)
        }
    });

    // 7. PUT Actuators Endpoint (Demonstrates logical state toggling using PUT for idempotency)
    let db_a_put = Arc::clone(&actuators_db);
    router = router.put("/api/actuators", move |ctx: RequestContext| {
        let db = Arc::clone(&db_a_put);
        async move {
            let body_str = ctx.req.body_as_str().unwrap_or_default();
            match serde_json::from_str::<ActuatorCmd>(body_str) {
                Ok(cmd) => {
                    let mut data = db.lock().unwrap();
                    if data.contains_key(&cmd.id) {
                        let new_state = cmd.command.to_uppercase() == "ON";
                        data.insert(cmd.id.clone(), new_state);
                        log::info!("Actuator {} turned {} via PUT", cmd.id, cmd.command);
                        
                        Response::new()
                            .with_header("Content-Type", "application/json")
                            .with_body(r#"{"status": "success"}"#)
                    } else {
                        Response::new().with_status(404, "Not Found").with_body("Actuator not found")
                    }
                }
                Err(_) => Response::bad_request()
            }
        }
    });

    // 8. Secure Endpoint (Demonstrates HTTP Header Reading Validation)
    router = router.get("/api/secure", move |ctx: RequestContext| {
        async move {
            let auth_header = ctx.req.header_value("Authorization");
            
            match auth_header {
                Some(token) if token == "Bearer secret_microgate_token_123" => {
                    Response::new()
                        .with_status(200, "OK")
                        .with_header("Content-Type", "application/json")
                        .with_body(r#"{"classified_data": "Protocol 7 enabled", "access": "granted"}"#)
                },
                _ => {
                    Response::new()
                        .with_status(401, "Unauthorized")
                        .with_header("WWW-Authenticate", "Bearer")
                        .with_header("Content-Type", "application/json")
                        .with_body(r#"{"error": "Missing or invalid Authorization header"}"#)
                }
            }
        }
    });

    // Start the server
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    
    let server = Server::new(&addr, router);
    log::info!("🚀 MicroGate Advanced Demo App listening on http://{}", addr);

    server.run().await
}
