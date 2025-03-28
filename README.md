# Torrust-Actix Tracker
![Test](https://github.com/Power2All/torrust-actix/actions/workflows/rust.yml/badge.svg)
[<img src="https://img.shields.io/badge/DockerHub-link-blue.svg">](<https://hub.docker.com/r/power2all/torrust-actix>)
[<img src="https://img.shields.io/discord/272362779157987328?label=Discord">](<https://discord.gg/ys9pb4w>)

## Project Description
Torrust-Actix Tracker is a lightweight but incredibly powerful and feature-rich BitTorrent Tracker made using Rust.

Currently, it's being actively used at https://www.gbitt.info/.

This project originated from Torrust-Tracker code originally developed by Mick van Dijke, further developed by Power2All as alternative for OpenTracker and other tracker code available on GitHub.

## Features
* [X] Block array for TCP tracking (HTTP/HTTPS), UDP tracking and API (HTTP/HTTPS)
* [X] Full IPv4 and IPv6 support
* [X] Persistence saving supported using SQLite3, MySQL or PostgresSQL database
* [X] Customize table and database structure in the configuration
* [X] Whitelist system for private tracking
* [X] Blacklist system for blocking unwelcome hashes
* [X] Torrent key support for locking access to announcement through keys as info_hash with a timeout
* [X] User account support, configurable for also database support
* [X] Swagger UI built-in in the API (toggleable), useful both for testing API and documentation for API
* [X] Sentry SaaS and self-hosted support

## Implemented BEPs
* [BEP 3](https://www.bittorrent.org/beps/bep_0003.html): The BitTorrent Protocol
* [BEP 7](https://www.bittorrent.org/beps/bep_0007.html): IPv6 Support
* [BEP 15](https://www.bittorrent.org/beps/bep_0015.html): UDP Tracker Protocol for BitTorrent
* [BEP 23](https://www.bittorrent.org/beps/bep_0023.html): Tracker Returns Compact Peer Lists
* [BEP 41](https://www.bittorrent.org/beps/bep_0041.html): UDP Tracker Protocol Extensions
* [BEP 48](https://www.bittorrent.org/beps/bep_0048.html): Tracker Protocol Extension: Scrape

## Getting Started
You can get the latest binaries from [releases](https://github.com/Power2All/torrust-actix/releases) or follow the install from scratch instructions below.

### Install From Scratch
1. Clone the repository:
```bash
git clone https://github.com/Power2All/torrust-actix.git
cd torrust-actix
```

2. Build the source code using Rust (make sure you have installed rustup with stable branch)
#### Using build script
```bash
cargo build --release
```

### Usage
Run the code using `--help` argument for using in your enironment:
```bash
./target/release/torrust-actix --help
```
Before you can run the server, you need to either have persietency turned off, and when enabled, make sure your database is created and working. See the help argument above how to fix your setup as you wish.

Swagger UI is introduced, and when enabled in the configuration, is accessible through the API via `/swagger-ui/`.

Sentry.io support is introduced, you can enable it in the configuration and the URL where to push the data to.

### Environment Variable Overrides

Use environment variables to override the configuration settings.

```
LOG_LEVEL <off | trace | debug | info | warn | error>
LOG_CONSOLE_INTERVAL <UINT64>

TRACKER__API_KEY <STRING>
TRACKER__WHITELIST_ENABLED <true | false>
TRACKER__BLACKLIST_ENABLED <true | false>
TRACKER__KEYS_ENABLED <true | false>
TRACKER__USERS_ENABLED <true | false>
TRACKER__SWAGGER <true | false>
TRACKER__KEYS_CLEANUP_INTERVAL <UINT64>
TRACKER__REQUEST_INTERVAL <UINT64>
TRACKER__REQUEST_INTERVAL_MINIMUM <UINT64>
TRACKER__PEERS_TIMEOUT <UINT64>
TRACKER__PEERS_CLEANUP_INTERVAL <UINT64>
TRACKER__PEERS_CLEANUP_THREADS <UINT64>
TRACKER__PROMETHEUS_ID <STRING>

SENTRY__ENABLED <true | false>
SENTRY__DEBUG <true | false>
SENTRY__ATTACH_STACKTRACE <true | false>
SENTRY__SEND_DEFAULT_PII <true | false>
SENTRY__DSN <STRING>
SENTRY__MAX_BREADCRUMBS <UINT64>
SENTRY__SAMPLE_RATE <F32>
SENTRY__TRACES_SAMPLE_RATE <F32>

DATABASE__PERSISTENT <true | false>
DATABASE__INSERT_VACANT <true | false>
DATABASE__REMOVE_ACTION <true | false>
DATABASE__UPDATE_COMPLETED <true | false>
DATABASE__UPDATE_PEERS <true | false>
DATABASE__PATH <STRING>
DATABASE__ENGINE <sqlite3 | mysql | pgsql>
DATABASE__PERSISTENT_INTERVAL <UINT64>

DATABASE_STRUCTURE__TORRENTS__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__TORRENTS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_INFOHASH <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_SEEDS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_PEERS <STRING>
DATABASE_STRUCTURE__TORRENTS__COLUMN_COMPLETED <STRING>

DATABASE_STRUCTURE__WHITELIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__WHITELIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__WHITELIST__COLUMN_INFOHASH <STRING>

DATABASE_STRUCTURE__BLACKLIST__BIN_TYPE_INFOHASH <true | false>
DATABASE_STRUCTURE__BLACKLIST__TABLE_NAME <STRING>
DATABASE_STRUCTURE__BLACKLIST__COLUMN_INFOHASH <STRING>

DATABASE_STRUCTURE__KEYS__BIN_TYPE_HASH <true | false>
DATABASE_STRUCTURE__KEYS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_HASH <STRING>
DATABASE_STRUCTURE__KEYS__COLUMN_TIMEOUT <STRING>

DATABASE_STRUCTURE__USERS__ID_UUID <true | false>
DATABASE_STRUCTURE__USERS__BIN_TYPE_KEY <true | false>
DATABASE_STRUCTURE__USERS__TABLE_NAME <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UUID <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_ID <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_ACTIVE <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_KEY <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UPLOADED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_DOWNLOADED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_COMPLETED <STRING>
DATABASE_STRUCTURE__USERS__COLUMN_UPDATED <STRING>
```

### ChangeLog

#### v4.0.10
* Updating libraries
* Adding full environment support to override configurations

#### v4.0.9
* Updating libraries (Actix 4.9 to 4.10)
* Some critical exploit in ZIP fixed
* Some faulty v4.0.8 deployments fixed with GitHub

#### v4.0.8
* Updating libraries
* Fixing threading for UDP
* Removed a feature from Parking Lot, cause of an unfixed vulnerability
* Update Swagger UI to version v4.0.0 and OpenAPI version v3.1.1

#### v4.0.7
* Cleanup was still broken, did a big rewrite, after testing it works now as expected
* Did some tokio threading correctly for core threads
* Added a new configuration key, to set the threads, default for each shard (256), but can be changed

#### v4.0.6
* Fixed some clippy issues
* Found a performance issue on peers cleanup
* Switched peers cleanup from Tokio spawn to Thread spawn for speedup
* Bumped version of Tokio

#### v4.0.5
* Library bump

#### v4.0.4
* Further implementation of Sentry (trace logging)

#### v4.0.3
* Fixing announce and scrape paths, since it is the wrong way.
* Fixing various smaller bugs that isn't app-breaking, but should be handled better.
* Added support for Sentry.io SaaS and self-hosted setup.
* Preparing work for version v4.1.0, which will introduce LUA support for middleware.

#### v4.0.2
* Added option that the system will remove data from database.
* Added updates variables for the white/black list and keys tables.
* Renaming the "database" naming which should be "tables".
* A lot of fixes and bugs I stumbled upon.

#### v4.0.0
* Completely rebuilt of the tracker code, for readability.
* Moved to Actix v4, thus versioning this software to v4.0.0 as well.
* Rebuilt and remade the way configuration file is created (you need to give the command as argument for it).
* Redone the whole database system, is tested with the latest versions available at this time.
* API has gone through a lot of work and tested.
* Introduced Swagger UI as testing and documentation.
* A lot of improvements in speed and performance applied further.
* Import and Export function added, will dump or import from JSON files, handy for when making a backup from your existing database, or when migrating to an other database engine.
* Removed WebGUI, was outdated and not really useful.

#### v3.2.2
* Bumped library versions significantly, including security patches.
* Fixed changes in libraries to work properly.
* Tuned the non-persistence code to use less memory.

#### v3.2.1
* Bumped library versions, including security patches.
* Fixed a bug in the PostgreSQL handler.
* Some forgotten naming from Torrust-Axum to Torrust-Actix.

#### v3.2.0
* Bumped library versions.
* Modified the way scheduling was done through threads, it could lock up and slow down public trackers with heavy activity.
* Tweaking the SQLite3 database usage and database space consumption.
* Full overhaul on how torrents and peers are used in memory. Using crossbeam skipmap for thread safe non-locking memory sharing.
* Some various improvement on coding performance, readability and linting the files.
* Replaced Tokio Axum web framework for Actix, reason: Missing critical things like a timeout on connect, disconnect, read and write, and support was lackluster.
* Renamed the GitHub repository from torrust-axum to torrust-actix.
* Adding user tracking support with an extra key.

#### v3.1.2
* Bumped library versions.
* Added a Code of Conduct file, as some open source projects need this.
* Added a Maintenance toggle function to API and WebGUI.
* Configuration file is not generated when it doesn't exist, or has invalid data, unless forced with a '--create-config' argument.
* Fixed various small bugs.

#### v3.1.1
* Bumped library versions.
* Database for SQLite3, MySQL and PostgreSQL now works properly with all the tables, and will be used if enabled.
* UDP had a problem in IPv4, fixed the code for correctly parsing byte array.
* Cleanup and refactoring of some redundant code.
* Added some small checks where needed to prevent errors.

#### v3.1.0
* Whitelist System: You can enable this to only allow torrent hashes to be used you specify in the database, or add them through the API.
* Blacklist System: You can enable this to disallow torrent hashes to be used you specify in the database, or add them through the API.
* Keys System: You can enable this to only allow tracking when an activated "key" hash (same as an info_hash, 20 bytes or 40 characters hex) is given. Keys with a timeout of zero "0" will be permanent and won't be purged by the cleanup.
* WebGUI: The API has an available web interface, which can be accessed through https://your.api:8080/webgui/ and giving the correct API Key, which you configure in the configuration file.
* Customizable database structure can be given in the configuration file.
* The system is also now available through Docker Hub at https://hub.docker.com/r/power2all/torrust-axum

#### v3.0.1
* Bugfixes
* SQLite3 support added
* MySQL support added
* PostgresSQL support added

#### v3.0.0
Initial version of Torrust-Axum.

### Credits
This Torrust-Tracker was a joint effort by [Nautilus Cyberneering GmbH](https://nautilus-cyberneering.de/), [Dutch Bits](https://dutchbits.nl) and [Power2All](https://power2all.com).
Also thanks to [Naim A.](https://github.com/naim94a/udpt) and [greatest-ape](https://github.com/greatest-ape/aquatic) for some parts in the Torrust-Tracker code.
This project (Torrust-Actix) is built from scratch by [Power2All](https://power2all.com).
