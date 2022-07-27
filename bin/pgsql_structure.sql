CREATE DATABASE gbitt
    WITH
    OWNER = postgres
    ENCODING = 'UTF8'
    LC_COLLATE = 'English_United States.1252'
    LC_CTYPE = 'English_United States.1252'
    TABLESPACE = pg_default
    CONNECTION LIMIT = -1;

CREATE TABLE IF NOT EXISTS public.torrents
(
    info_hash bytea NOT NULL,
    completed bigint NOT NULL DEFAULT 0,
    CONSTRAINT torrents_pkey PRIMARY KEY (info_hash)
    )

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.torrents
    OWNER to postgres;