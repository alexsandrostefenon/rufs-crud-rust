# rufs-crud-es6

Restful Utilities for Full Stack - CRUD WebApp

You need Rust + wasm-bindgen installed and PostgreSql server already running with your database.

Requires Rust version >= 1.63

Requires browser with support to dynamic ES6 modules (tested with Chrome versions >= 64)

## First Step

Open terminal and clone this repository with `git clone https://github.com/alexsandrostefenon/rufs-crud-rust`.

To download the required dependencies and build, then

`wasm-pack build --target web --dev` 

### Run Ecosystem

expose database information, like :

export PGHOST=localhost;
export PPORT=5432;
export PGDATABASE=<database name>;
export PGUSER=<database user>;
export PGPASSWORD=<database password>;

## Web application

In ES6 compliance browser open url

`http://localhost:8080/crud`

For custom service configuration or user edition, use user 'admin' with password 'admin'.
