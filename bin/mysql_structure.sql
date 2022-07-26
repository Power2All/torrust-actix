CREATE DATABASE IF NOT EXISTS `gbitt`;
USE `gbitt`;

CREATE TABLE IF NOT EXISTS `torrents` (
    `info_hash` binary(20) NOT NULL,
    `completed` int NOT NULL DEFAULT '0',
    PRIMARY KEY (`info_hash`) USING BTREE,
    UNIQUE KEY `info_hash` (`info_hash`)
    ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_general_ci;

