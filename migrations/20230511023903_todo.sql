-- Add migration script here
CREATE TABLE todo
(
    id          SERIAL PRIMARY KEY,
    title       VARCHAR(255) NOT NULL,
    description text,
    completed   boolean      NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP    NOT NULL DEFAULT now(),
    updated_at  TIMESTAMP    NOT NULL DEFAULT now()
)
