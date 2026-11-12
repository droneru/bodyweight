-- +goose Up
CREATE TABLE users (
    login Utf8 NOT NULL,
    yandex_uid Utf8,
    display_name Utf8 NOT NULL,
    avatar_id Utf8,
    PRIMARY KEY (login)
);

CREATE TABLE sessions (
    id Utf8 NOT NULL,
    user_login Utf8 NOT NULL,
    expires_at Timestamp NOT NULL,
    PRIMARY KEY (id)
) WITH (TTL = Interval("PT0S") ON expires_at);

-- +goose Down
DROP TABLE sessions;

DROP TABLE users;
