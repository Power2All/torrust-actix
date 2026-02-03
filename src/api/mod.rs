//! REST API module for tracker management and statistics.
//!
//! This module provides HTTP endpoints for managing the tracker through a REST API.
//! It supports operations on torrents, users, whitelists, blacklists, API keys,
//! and provides statistics and monitoring endpoints.
//!
//! # Endpoints Overview
//!
//! ## Statistics
//! - `GET /stats` - Get tracker statistics in JSON format
//! - `GET /metrics` - Get Prometheus-format metrics
//!
//! ## Torrents
//! - `GET /api/torrent/{info_hash}` - Get torrent information
//! - `POST /api/torrent/{info_hash}/{completed}` - Add/update torrent
//! - `DELETE /api/torrent/{info_hash}` - Delete torrent
//! - `GET /api/torrents` - List all torrents
//! - `POST /api/torrents` - Batch add/update torrents
//! - `DELETE /api/torrents` - Batch delete torrents
//!
//! ## Whitelist
//! - `GET /api/whitelist/{info_hash}` - Check if hash is whitelisted
//! - `POST /api/whitelist/{info_hash}` - Add hash to whitelist
//! - `DELETE /api/whitelist/{info_hash}` - Remove from whitelist
//! - `GET /api/whitelists` - List all whitelisted hashes
//!
//! ## Blacklist
//! - `GET /api/blacklist/{info_hash}` - Check if hash is blacklisted
//! - `POST /api/blacklist/{info_hash}` - Add hash to blacklist
//! - `DELETE /api/blacklist/{info_hash}` - Remove from blacklist
//! - `GET /api/blacklists` - List all blacklisted hashes
//!
//! ## API Keys
//! - `GET /api/key/{key_hash}` - Get key information
//! - `POST /api/key/{key_hash}/{timeout}` - Create key with timeout
//! - `DELETE /api/key/{key_hash}` - Delete key
//! - `GET /api/keys` - List all keys
//!
//! ## Users
//! - `GET /api/user/{id}` - Get user information
//! - `POST /api/user/{id}/{key}/{uploaded}/{downloaded}/{completed}/{updated}/{active}` - Create/update user
//! - `DELETE /api/user/{id}` - Delete user
//! - `GET /api/users` - List all users
//!
//! ## SSL Certificate Management
//! - `POST /api/certificate/reload` - Hot-reload SSL certificates
//! - `GET /api/certificate/status` - Get certificate status
//!
//! # Authentication
//!
//! All API endpoints require a valid API token passed as a query parameter:
//! `?token=<api_key>`

/// Data structures for API service context.
pub mod structs;

/// Core API service functions and route configuration.
#[allow(clippy::module_inception)]
pub mod api;

/// Blacklist management endpoints.
pub mod api_blacklists;

/// SSL certificate management endpoints.
pub mod api_certificate;

/// API key management endpoints.
pub mod api_keys;

/// Torrent management endpoints.
pub mod api_torrents;

/// User management endpoints.
pub mod api_users;

/// Whitelist management endpoints.
pub mod api_whitelists;

/// Statistics and monitoring endpoints.
pub mod api_stats;