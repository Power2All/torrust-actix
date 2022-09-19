CREATE DATABASE IF NOT EXISTS `gbitt`;
USE `gbitt`;

CREATE TABLE IF NOT EXISTS `torrents` (
    `info_hash` binary(20) NOT NULL,
    `completed` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `whitelist` (
    `info_hash` binary(20) NOT NULL,
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `blacklist` (
    `info_hash` binary(20) NOT NULL,
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

CREATE TABLE IF NOT EXISTS `keys` (
    `hash` binary(20) NOT NULL,
    `timeout` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`hash`) USING BTREE,
    UNIQUE KEY `hash` (`hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;
