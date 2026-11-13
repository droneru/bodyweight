-- +goose Up
CREATE TABLE workouts (
    user_login Utf8 NOT NULL,
    id Utf8 NOT NULL,
    date Date NOT NULL,
    note Utf8 NOT NULL,
    created_at Timestamp NOT NULL,
    PRIMARY KEY (user_login, id)
);

CREATE TABLE exercise_entries (
    user_login Utf8 NOT NULL,
    workout_id Utf8 NOT NULL,
    position Uint32 NOT NULL,
    exercise_id Utf8 NOT NULL,
    PRIMARY KEY (user_login, workout_id, position)
);

CREATE TABLE sets (
    user_login Utf8 NOT NULL,
    workout_id Utf8 NOT NULL,
    entry_position Uint32 NOT NULL,
    position Uint32 NOT NULL,
    reps Uint32 NOT NULL,
    weight_quarters Uint32,
    PRIMARY KEY (user_login, workout_id, entry_position, position)
);

-- +goose Down
DROP TABLE sets;

DROP TABLE exercise_entries;

DROP TABLE workouts;
