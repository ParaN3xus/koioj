# Setup

- Install [PostgreSQL](https://www.postgresql.org/).
- Create a user, a database `theoj_db` under this user, and import our schema by
  ```sh
  psql -U postgres -d theoj_db -f schema.sql
  ```
- Create `.env` and `config.yml` with given template.
- Generate `theoj-api` TypeScript library by `yarn generate-openapi`.
- Run `theoj-api` by `cargo run --bin theoj-api`.
