# Setup

## Setup `koioj-judge`

- Generate an SSH key pair for authentication with `koioj-api` (without passphrase).
- Create `judge_config.yml` with given template.
- Install sandbox with `sudo ./koioj-judge -c judger_config.yml install-sandbox`
- Run `koioj-judge` by `systemd-run --user --scope -p Delegate=yes -- ./koioj-judge -c judger_config.yml serve`

## Setup `koioj-api`

- Install [PostgreSQL](https://www.postgresql.org/).
- Create a user, a database `koioj_db` under this user, and import our schema by
  ```sh
  psql -U postgres -d koioj_db -f schema.sql
  ```
- Obtain the public key from `koioj-judge` for authentication.
- Create `.env` and `config.yml` with given template.
- Generate `koioj-api` TypeScript library by `yarn generate-openapi`.
- Run `koioj-api` by `cargo run --bin koioj-api`.
