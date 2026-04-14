# MicroGate Web Application Demo

This is a demonstration web application showcasing the full capabilities of the **MicroGate** HTTP framework. It perfectly simulates an IoT Sensor & Actuators Dashboard, displaying real-time data fetched from the backend, and allows hardware simulating sensors to interact utilizing HTTP/1.1 mechanics out-of-the-box.

The framework dependency (MicroGate) is now fetched securely from the cloud (GitHub), making this project entirely standalone and portable.

## Features Showcased
- **Cloud Dependency Management**: Demonstrates pulling the `MicroGate` framework directly from a GitHub repository, ensuring portability.
- **Static File Serving**: Serves the frontend assets (HTML, CSS, JS) efficiently via `fs::serve_file`.
- **Full CRUD REST API Mapping**: Showcases handling of all major HTTP methods: `GET`, `POST`, `PUT`, `DELETE`, and `PATCH`.
- **Global Static State Management**: Utilizes `std::sync::OnceLock` and `Arc<Mutex<T>>` to manage thread-safe global states across async requests.
- **Real-time Server Analytics**: Integrates the `sysinfo` crate to provide live CPU and RAM resource tracking via the `/api/system` endpoint.
- **Header Security & Validations**: Demonstrates extracting and validating HTTP headers (e.g., `Authorization: Bearer <token>`) to protect secure endpoints (`/api/secure`).
- **MicroGate & WireFrame limits**: Natively utilizes strict parsing, correctly separating headers and bodies over standard single-connection TCP contexts (HTTP/1.1).

---

## 🚀 How to Deploy (For Non-Technical Users)

You can easily deploy and run this application using **Docker**, meaning you don't need to install Rust or any build tools locally on your system—Docker will handle everything for you.

### Prerequisites
1. Download and install [Docker Desktop](https://www.docker.com/products/docker-desktop/).
2. Keep Docker running in the background.

### Deployment Steps
1. Open your terminal natively (Command Prompt / Powershell on Windows, Terminal on Mac/Linux).
2. Navigate to the directory containing `microgate-demo`.
    ```bash
    cd /path/to/your/repository/microgate-demo
    ```
3. Build the Docker image:
    ```bash
    docker build -t microgate-demo-app .
    ```
4. Start the container in the background:
    ```bash
    docker run -d -p 8080:8080 --name my-microgate-app microgate-demo-app
    ```
5. Open your web browser and go to: [http://localhost:8080](http://localhost:8080)
   You're live!

---

## 🧪 How to Test & Prove HTTP/1.1 Features

To verify the app is really handling standard HTTP/1.1 specifications (including Chunked Encoding, JSON serialization, and dynamic Routing), use **Postman** or the **built-in web dashboard simulator**.

### 1. Test Static files mapping (GET)
- **Open Browser**: `http://localhost:8080/index.html`
- **What it Proves**: `MicroGate` reliably traverses local directories via the native `fs` implementation to deliver HTML properly bound with `Content-Type: text/html`.

### 2. Test Reading API state (GET)
- **Postman Command**: Send a `GET` request to `http://localhost:8080/api/system`.
- **What it Proves**: The framework effortlessly routes dynamic REST endpoints and serves structured JSON data built via `Response::new().with_header(...)`. It also shows real-time RAM/CPU tracking.

### 3. Test Updating State (POST, PUT, DELETE)
- **Dashboard Usage**: Use the Simulator Panel to Add/Update sensors (POST), toggle Actuators (PUT), and remove sensors (DELETE).
- **What it Proves**: `MicroGate` parses paths and correctly delegates requests to the proper handler based on the HTTP method (`router.get()`, `router.post()`, `router.put()`, `router.delete()`).

### 5. Test HTTP/1.1 Chunked Transfer Encoding (Advanced Proof)
HTTP/1.1 requires servers to process `Transfer-Encoding: chunked` bodies natively without explicit Content-Length.
Using `curl`, we can forcefully send chunked HTTP/1.1 requests.

- **Command**:
  ```bash
  curl -X POST -H "Content-Type: application/json" -H "Transfer-Encoding: chunked" -d '{"id": "chunk_sensor", "value": 99.9}' http://localhost:8080/api/sensors
  ```
- **What it Proves**: The HTTP parser & framework behind `MicroGate` is a true compliant HTTP/1.1 byte-streaming state-machine, successfully unpacking variable-length chunks on fly!