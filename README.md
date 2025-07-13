# Lightning Network Node Server

A simple web server that fetches Lightning Network node data from an external API, stores it in a local database, and exposes it via a JSON API.

## Build tools & versions used

*   **Language:** Rust
*   **Containerization:** Docker
*   **Key Libraries:**
    *   `actix-web` for the HTTP server.
    *   `tokio` as the asynchronous runtime.
    *   `rusqlite` for the SQLite database.
    *   `reqwest` for making HTTP requests.
    *   `moka` for in-memory caching.
    *   `dotenvy` for environment variable management.

## Steps to run the app

You need Docker installed to run this application.

1.  **Build the Docker image:**
    ```sh
    docker build -t ln-nodes-server .
    ```

2.  **Run the Docker container:**
    ```sh
    docker run -p 8080:8080 --name ln-nodes-app ln-nodes-server
    ```
    The server will be available at `http://localhost:8080`. The first time you run it, a `.env` file with default settings is created inside the container.

3.  **Access the API:**
    You can get the node data by making a GET request to `http://localhost:8080/nodes`.
    ```sh
    curl http://localhost:8080/nodes
    ```

## Design Philosophy

The technical decisions for this project were guided by a minimalist approach suitable for the small scope of the application. The main goal was to provide a simple, robust, and efficient solution without over-engineering.

For example:
*   **Database:** SQLite was chosen for its simplicity and because it runs in-process, avoiding the need for a separate database server.
*   **Caching:** The in-memory cache (`moka`) is sufficient for improving performance without the complexity of an external service like Redis.
*   **Configuration:** Environment variables (`.env`) provide enough flexibility for this scale.
