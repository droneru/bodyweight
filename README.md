# Bodyweight

A strength-training log at [bodyweight.ru](https://bodyweight.ru). Record workouts and weigh-ins, and track PRs in pull-ups and the Big Three (bench press, squat, deadlift) over the last 90 days and all time.

- Mobile-first web app.
- Admin CLI that works directly with the database.


## Development

Requires docker, a Rust toolchain and node.

```sh
make dev                      # YDB in docker + server + Vite: http://localhost:5173
                              # sign in with the "Dev sign in" form under any login
make dev-db                   # only YDB with migrations applied
make cli                      # build target/release/bodyweight
eval "$(make -s dev-env)"     # fish: make -s dev-env | source
export BODYWEIGHT_USER=alice  # Yandex login to act on behalf of

target/release/bodyweight weigh-ins add 82.5
target/release/bodyweight weigh-ins list

make dev-ydb                  # interactive YQL shell on the dev YDB
make dev-ydb ARGS="sql -s 'SELECT * FROM weigh_ins'"

make dev-down                 # stop, keep data; make dev-clean also deletes it
```

`make dev` signs users in without OAuth (`BODYWEIGHT_DEV_LOGIN=1`). To try the real
sign-in with Yandex ID locally, add `http://localhost:5173/api/auth/yandex/callback` to the
Redirect URIs of the OAuth application and export `BODYWEIGHT_YANDEX_CLIENT_ID` and
`BODYWEIGHT_YANDEX_CLIENT_SECRET` before `make dev`.

## Deployment

The app needs a Serverless YDB database, a service account (**sa**) to connect to it, and a
Yandex OAuth application for sign-in.

One-time setup:

1. Give the sa the `ydb.editor` role on the database (or on its folder).
2. Create an authorized key of the sa and put it on the VPS as `deploy/secrets/sa-key.json`:
   ```sh
   yc iam key create --service-account-name <sa name> -o deploy/secrets/sa-key.json
   ```
   The app runs as uid 65532 and must be able to read it:
   `sudo chown 65532 deploy/secrets/sa-key.json && sudo chmod 400 deploy/secrets/sa-key.json`.
3. Register an application at [oauth.yandex.ru](https://oauth.yandex.ru) with the Redirect URI
   `https://bodyweight.ru/api/auth/yandex/callback` and the permissions `login:info` and
   `login:avatar`.
4. Copy `deploy/.env.example` to `deploy/.env` on the VPS and fill in the YDB location and
   credentials and the ClientID and Client secret of the application.

Before deploying a version with new migrations, apply them (from any machine with docker):

```sh
YDB_ENDPOINT=grpcs://ydb.serverless.yandexcloud.net:2135 YDB_DATABASE=/ru-central1/... \
YDB_ACCESS_TOKEN_CREDENTIALS=$(yc iam create-token) make migrate
```

Then on the VPS, from a clone of the repository (only docker is needed):

```sh
make deploy                   # git pull, rebuild the app image, restart the app
```

HTTPS is served by a shared reverse proxy on the VPS (e.g. Caddy), which obtains
certificates automatically and proxies bodyweight.ru to `bodyweight-app:3000`
over the external docker network `proxy`. Ports 80 and 443 must be open and the domain
must point at the VPS.

## Docs

- [CONTEXT.md](CONTEXT.md) — domain glossary
- [docs/spec.md](docs/spec.md) — specification (in Russian)

## License

See [LICENSE](LICENSE).
