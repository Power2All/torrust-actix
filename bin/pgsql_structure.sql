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

CREATE TABLE IF NOT EXISTS public.whitelist
(
    info_hash bytea NOT NULL,
    CONSTRAINT whitelist_pkey PRIMARY KEY (info_hash)
    )

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.whitelist
    OWNER to postgres;

CREATE TABLE IF NOT EXISTS public.blacklist
(
    info_hash bytea NOT NULL,
    CONSTRAINT blacklist_pkey PRIMARY KEY (info_hash)
    )

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.blacklist
    OWNER to postgres;

CREATE TABLE IF NOT EXISTS public.keys
(
    hash bytea NOT NULL,
    timeout bigint NOT NULL DEFAULT 0,
    CONSTRAINT keys_pkey PRIMARY KEY (hash)
    )

    TABLESPACE pg_default;

ALTER TABLE IF EXISTS public.keys
    OWNER to postgres;
