# Bodyweight

A strength-training log at [bodyweight.ru](https://bodyweight.ru). Record workouts and weigh-ins, and track PRs in pull-ups and the Big Three (bench press, squat, deadlift) over the last 90 days and all time.

- Mobile-first web app.
- Admin CLI that works directly with the database.


## Development

Requires docker and a Rust toolchain.

```sh
make dev                      # start YDB in docker and apply migrations
make cli                      # build target/release/bodyweight
eval "$(make -s dev-env)"     # fish: make -s dev-env | source
export BODYWEIGHT_USER=alice  # Yandex login to act on behalf of

target/release/bodyweight weigh-ins add 82.5
target/release/bodyweight weigh-ins list

make dev-ydb                  # interactive YQL shell on the dev YDB
make dev-ydb ARGS="sql -s 'SELECT * FROM weigh_ins'"

make dev-down                 # stop, keep data; make dev-clean also deletes it
```

## Docs

- [CONTEXT.md](CONTEXT.md) — domain glossary
- [docs/spec.md](docs/spec.md) — specification (in Russian)

## License

See [LICENSE](LICENSE).
