-- +goose Up
CREATE TABLE custom_exercises (
    user_login Utf8 NOT NULL,
    id Utf8 NOT NULL,
    name Utf8 NOT NULL,
    archived Bool NOT NULL,
    PRIMARY KEY (user_login, id)
);

-- +goose Down
DROP TABLE custom_exercises;
