// Performance benchmarks for Torrust-Actix
// Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use torrust_actix::common::structs::number_of_bytes::NumberOfBytes;
use torrust_actix::config::structs::configuration::Configuration;
use torrust_actix::tracker::enums::announce_event::AnnounceEvent;
use torrust_actix::tracker::enums::torrent_peers_type::TorrentPeersType;
use torrust_actix::tracker::structs::info_hash::InfoHash;
use torrust_actix::tracker::structs::peer_id::PeerId;
use torrust_actix::tracker::structs::torrent_peer::TorrentPeer;
use torrust_actix::tracker::structs::torrent_tracker::TorrentTracker;

fn random_info_hash() -> InfoHash {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.gen();
    InfoHash(bytes)
}

fn random_peer_id() -> PeerId {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: [u8; 20] = rng.gen();
    PeerId(bytes)
}

fn create_test_peer(ip: IpAddr, port: u16) -> TorrentPeer {
    TorrentPeer {
        peer_addr: std::net::SocketAddr::new(ip, port),
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(0),
        downloaded: NumberOfBytes(0),
        left: NumberOfBytes(1000),
        event: AnnounceEvent::Started,
    }
}

async fn create_tracker() -> Arc<TorrentTracker> {
    let mut config = Configuration::default();
    config.database.persistent = false;
    Arc::new(TorrentTracker::new(Arc::new(config), false).await)
}

fn bench_add_peer(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());

    c.bench_function("add_peer", |b| {
        b.iter(|| {
            let info_hash = random_info_hash();
            let peer_id = random_peer_id();
            let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
            black_box(tracker.add_torrent_peer(info_hash, peer_id, peer, false));
        });
    });
}

fn bench_get_peers_with_limit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());
    let info_hash = random_info_hash();

    // Pre-populate with 1000 peers
    for i in 0..1000 {
        let peer_id = random_peer_id();
        let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)), 6881);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }

    let mut group = c.benchmark_group("get_peers_with_early_exit");

    for limit in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(limit), limit, |b, &limit| {
            b.iter(|| {
                black_box(tracker.get_torrent_peers(
                    info_hash,
                    limit,
                    TorrentPeersType::IPv4,
                    None,
                ));
            });
        });
    }

    group.finish();
}

fn bench_concurrent_peer_additions(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("concurrent_100_peers", |b| {
        b.iter(|| {
            rt.block_on(async {
                let tracker = create_tracker().await;
                let info_hash = random_info_hash();

                let mut handles = vec![];
                for i in 0..100 {
                    let tracker_clone = tracker.clone();
                    let handle = tokio::spawn(async move {
                        let peer_id = random_peer_id();
                        let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i)), 6881);
                        tracker_clone.add_torrent_peer(info_hash, peer_id, peer, false);
                    });
                    handles.push(handle);
                }

                for handle in handles {
                    handle.await.unwrap();
                }
            });
        });
    });
}

fn bench_sharding_distribution(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());

    c.bench_function("shard_access_256_torrents", |b| {
        b.iter(|| {
            for _ in 0..256 {
                let info_hash = random_info_hash();
                let peer_id = random_peer_id();
                let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881);
                black_box(tracker.add_torrent_peer(info_hash, peer_id, peer, false));
            }
        });
    });
}

fn bench_udp_packet_parsing(c: &mut Criterion) {
    use byteorder::{BigEndian, WriteBytesExt};
    use torrust_actix::udp::enums::request::Request;
    use torrust_actix::udp::udp::PROTOCOL_IDENTIFIER;

    let mut packet = vec![];
    packet.write_u64::<BigEndian>(PROTOCOL_IDENTIFIER).unwrap();
    packet.write_u32::<BigEndian>(0).unwrap(); // Connect action
    packet.write_u32::<BigEndian>(12345).unwrap(); // Transaction ID

    c.bench_function("udp_connect_request_parse", |b| {
        b.iter(|| {
            // This benchmarks the zero-copy optimization
            black_box(Request::from_bytes(&packet[..], 74));
        });
    });
}

fn bench_peer_filtering_ipv4_vs_ipv6(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());
    let info_hash = random_info_hash();

    // Add mixed IPv4 and IPv6 peers
    for i in 0..500 {
        let peer_id = random_peer_id();
        let peer = if i % 2 == 0 {
            create_test_peer(IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)), 6881)
        } else {
            create_test_peer(IpAddr::V6(format!("2001:db8::{:x}::{:x}", i / 256, i % 256).parse().unwrap()), 6881)
        };
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }

    let mut group = c.benchmark_group("peer_filtering");

    group.bench_function("ipv4_only", |b| {
        b.iter(|| {
            black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::IPv4, None));
        });
    });

    group.bench_function("ipv6_only", |b| {
        b.iter(|| {
            black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::IPv6, None));
        });
    });

    group.bench_function("all_types", |b| {
        b.iter(|| {
            black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::All, None));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_add_peer,
    bench_get_peers_with_limit,
    bench_concurrent_peer_additions,
    bench_sharding_distribution,
    bench_udp_packet_parsing,
    bench_peer_filtering_ipv4_vs_ipv6,
);

criterion_main!(benches);
