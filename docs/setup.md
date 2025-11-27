# Setup

## Setup `theoj-judge`

- Generate an SSH key pair for authentication with `theoj-api` (without passphrase).
- Create `judge_config.yml` with given template.
- Install sandbox with `sudo ./theoj-judge -c judger_config.yml install-sandbox`
- Run `theoj-judge` by `systemd-run --user --scope -p Delegate=yes -- ./theoj-judge -c judger_config.yml serve`

## Setup `theoj-api`

- Install [PostgreSQL](https://www.postgresql.org/).
- Create a user, a database `theoj_db` under this user, and import our schema by
  ```sh
  psql -U postgres -d theoj_db -f schema.sql
  ```
- Obtain the public key from `theoj-judge` for authentication.
- Create `.env` and `config.yml` with given template.
- Generate `theoj-api` TypeScript library by `yarn generate-openapi`.
- Run `theoj-api` by `cargo run --bin theoj-api`.
