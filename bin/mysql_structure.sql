CREATE DATABASE IF NOT EXISTS `gbitt`;
USE `gbitt`;

CREATE TABLE IF NOT EXISTS `torrents` (
    `info_hash` varchar(40) NOT NULL,
    `completed` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `whitelist` (
    `info_hash` varchar(40) NOT NULL,
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `blacklist` (
    `info_hash` varchar(40) NOT NULL,
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `keys` (
    `hash` varchar(40) NOT NULL,
    `timeout` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`hash`) USING BTREE,
    UNIQUE KEY `hash` (`hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `users` (
    `uuid` varchar(36) NOT NULL,
    `key` varchar(40) NOT NULL,
    `uploaded` int NOT NULL DEFAULT '0',
    `downloaded` int NOT NULL DEFAULT '0',
    `completed` int NOT NULL DEFAULT '0',
    `updated` int NOT NULL DEFAULT '0',
    `active` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`uuid`) USING BTREE,
    UNIQUE KEY `uuid` (`uuid`),
    UNIQUE KEY `key` (`key`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
