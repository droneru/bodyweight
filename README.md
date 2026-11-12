# Bodyweight

A strength-training log at [bodyweight.ru](https://bodyweight.ru). Record workouts and weigh-ins, and track PRs in pull-ups and the Big Three (bench press, squat, deadlift) over the last 90 days and all time.

- Mobile-first web app.
- Admin CLI that works directly with the database.


## Development

Requires docker, a Rust toolchain and node.

```sh
make dev                      # YDB in docker + server + Vite: http://localhost:5173
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

## Deployment

On the VPS, from a clone of the repository (only docker is needed):

```sh
make deploy                   # git pull, rebuild the app image, restart the app
```

HTTPS is served by the shared Caddy on the VPS (`/root/src/compose/caddy`), which
obtains certificates automatically and proxies bodyweight.ru to `bodyweight-app:3000`
over the external docker network `proxy`. Ports 80 and 443 must be open and the domain
must point at the VPS.

## Docs

- [CONTEXT.md](CONTEXT.md) — domain glossary
- [docs/spec.md](docs/spec.md) — specification (in Russian)

## License

See [LICENSE](LICENSE).
