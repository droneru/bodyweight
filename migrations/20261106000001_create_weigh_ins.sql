-- +goose Up
CREATE TABLE weigh_ins (
    user_login Utf8 NOT NULL,
    date Date NOT NULL,
    bodyweight_tenths Uint32 NOT NULL,
    PRIMARY KEY (user_login, date)
);

-- +goose Down
DROP TABLE weigh_ins;
