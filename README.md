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

## What was the reason for your focus? What problems were you trying to solve?

The focus was on creating a simple, robust, and efficient solution that directly meets the requirements without over-engineering. The main problem was to build a reliable service that could continuously ingest data from an external source and serve it through a resilient API that would not crash under any circumstances.

## How long did you spend on this project?

2 Hours

## Did you make any trade-offs for this project? What would you have done differently with more time?

Yes, several trade-offs were made to keep the design minimalist and aligned with the project's scope:

*   **Database:** I chose SQLite for its simplicity and because it runs in-process, avoiding the complexity of setting up and managing a separate database server like PostgreSQL.
*   **Caching:** I used a simple in-memory cache (`moka`). It is sufficient for this scope and avoids the overhead of an external service like Redis.

With more time, I would have added a comprehensive test suite, including unit tests for the data formatters and integration tests for the API endpoint.

## What do you think is the weakest part of your project?

The project's main weakness is also its strength: its minimalist design. While it is well-suited for the specific task, it lacks some features that would be necessary for a larger-scale, production application, such as a more comprehensive test suite.

## Is there any other information you’d like us to know?

The project's architecture was intentionally kept simple to avoid premature optimization, which I believe can be harmful if applied without a clear need. Solutions should be tailored to the problem at hand.

For a project with a long-term vision, it is important to plan for expansion and choose more scalable technologies. However, for this project, the goal was to build a clean and reliable solution that perfectly fits the defined scope.
