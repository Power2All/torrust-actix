use criterion::{
    criterion_group,
    criterion_main,
    BenchmarkId,
    Criterion
};
use rand::RngExt;
use std::net::{
    IpAddr,
    Ipv4Addr
};
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
    let mut rng = rand::rng();
    let bytes: [u8; 20] = rng.random();
    InfoHash(bytes)
}

fn random_peer_id() -> PeerId {
    let mut rng = rand::rng();
    let bytes: [u8; 20] = rng.random();
    PeerId(bytes)
}

fn create_test_peer(ip: IpAddr, port: u16, peer_id: PeerId) -> TorrentPeer {
    TorrentPeer {
        peer_id,
        peer_addr: std::net::SocketAddr::new(ip, port),
        updated: std::time::Instant::now(),
        uploaded: NumberOfBytes(0),
        downloaded: NumberOfBytes(0),
        left: NumberOfBytes(1000),
        event: AnnounceEvent::Started,
        rtc_data: None,
    }
}

async fn create_tracker() -> Arc<TorrentTracker> {
    let mut config = Configuration::init();
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
            let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881, peer_id);
            std::hint::black_box(tracker.add_torrent_peer(info_hash, peer_id, peer, false));
        });
    });
}

/// Re-announces into a swarm that already has peers.
///
/// [`bench_add_peer`] draws a fresh info-hash every iteration, so it only ever takes
/// `add_torrent_peer`'s vacant branch and snapshots a one-peer torrent. Production announces take
/// the occupied branch and pay for a full `AnnounceEntry::from_entry` over a populated swarm,
/// which is the cost that actually scales.
fn bench_announce_into_populated_swarm(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());

    let mut group = c.benchmark_group("announce_into_populated_swarm");
    for swarm_size in [10usize, 200, 1000].iter() {
        let info_hash = random_info_hash();
        // Half seeds, half leechers, so both maps the response reads are populated.
        for i in 0..*swarm_size {
            let peer_id = random_peer_id();
            let mut peer = create_test_peer(
                IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)),
                6881,
                peer_id
            );
            if i % 2 == 0 {
                peer.left = NumberOfBytes(0);
            }
            tracker.add_torrent_peer(info_hash, peer_id, peer, false);
        }
        // One peer id, re-announced: the steady-state case for a client on its announce timer.
        let returning_peer_id = random_peer_id();
        group.bench_with_input(BenchmarkId::from_parameter(swarm_size), swarm_size, |b, _| {
            b.iter(|| {
                let peer = create_test_peer(
                    IpAddr::V4(Ipv4Addr::new(10, 1, 1, 1)),
                    6881,
                    returning_peer_id
                );
                std::hint::black_box(tracker.add_torrent_peer(info_hash, returning_peer_id, peer, false));
            });
        });
    }
    group.finish();
}

fn bench_get_peers_with_limit(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());
    let info_hash = random_info_hash();
    for i in 0..1000 {
        let peer_id = random_peer_id();
        let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)), 6881, peer_id);
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    let mut group = c.benchmark_group("get_peers_with_early_exit");
    for limit in [10, 50, 100, 200].iter() {
        group.bench_with_input(BenchmarkId::from_parameter(limit), limit, |b, &limit| {
            b.iter(|| {
                std::hint::black_box(tracker.get_torrent_peers(
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
                        let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i)), 6881, peer_id);
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

/// Announces from many threads to *different* torrents.
///
/// Distinct info-hashes land on distinct shards, so the shard locks never contend and the only
/// state every thread shares is the global statistics counters. That isolates the cache-line
/// question `PaddedCounter` exists to answer: a single announce writes seeds, peers, the update
/// queue length and a per-protocol tally, and packed into one or two lines every one of those
/// writes invalidates the line on every other core.
fn bench_contended_stats(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());
    // Pre-create the torrents so the measured loop takes the occupied branch and never allocates
    // a new swarm, leaving the counters as the dominant shared write.
    const THREADS: usize = 8;
    // A multiple of THREADS so the per-task slices below divide evenly.
    let hashes: Arc<Vec<(InfoHash, PeerId)>> = Arc::new((0..64u8)
        .map(|i| {
            let mut raw = [0u8; 20];
            raw[0] = i;
            raw[1] = 0x5a;
            let info_hash = InfoHash(raw);
            let peer_id = PeerId(raw);
            let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(10, 2, 0, i)), 6881, peer_id);
            tracker.add_torrent_peer(info_hash, peer_id, peer, false);
            (info_hash, peer_id)
        })
        .collect());

    c.bench_function("contended_stats_8_threads", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(THREADS);
                for thread in 0..THREADS {
                    let tracker = tracker.clone();
                    let hashes = Arc::clone(&hashes);
                    handles.push(tokio::spawn(async move {
                        // Each task owns a disjoint slice of the hashes, so no two tasks ever
                        // touch the same info-hash and the shard write locks stay uncontended.
                        // Cycling the whole set instead — as this did — put every task on every
                        // shard, and the lock contention swamped the counter traffic this is
                        // meant to isolate.
                        let per_thread = hashes.len() / THREADS;
                        let owned = &hashes[thread * per_thread..(thread + 1) * per_thread];
                        for step in 0..250usize {
                            let (info_hash, peer_id) = owned[step % owned.len()];
                            let peer = create_test_peer(
                                IpAddr::V4(Ipv4Addr::new(10, 2, 1, thread as u8)),
                                6881,
                                peer_id
                            );
                            tracker.add_torrent_peer(info_hash, peer_id, peer, false);
                        }
                    }));
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
                let peer = create_test_peer(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 6881, peer_id);
                std::hint::black_box(tracker.add_torrent_peer(info_hash, peer_id, peer, false));
            }
        });
    });
}

/// Parses a realistic HTTP announce query string.
///
/// `parse_query` allocates a lower-cased `String` per key and a `Vec<u8>` per value, and is the
/// last hot-path map still on SipHash. This measures whether that is worth restructuring, given
/// that UDP never reaches it and actix does its own per-request work around it.
fn bench_parse_query(c: &mut Criterion) {
    use torrust_actix::common::common::parse_query;

    // Percent-encoded binary info_hash and peer_id, as a real client sends them.
    let query = "info_hash=%12%34%56%78%9a%bc%de%f0%12%34%56%78%9a%bc%de%f0%12%34%56%78\
                 &peer_id=-TR3000-abcdefghijkl\
                 &port=6881&uploaded=0&downloaded=0&left=1024&numwant=50&compact=1&event=started";
    c.bench_function("parse_announce_query", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(parse_query(Some(std::hint::black_box(query))));
        });
    });
}

fn bench_udp_packet_parsing(c: &mut Criterion) {
    use torrust_actix::udp::enums::request::Request;
    use torrust_actix::udp::udp::PROTOCOL_IDENTIFIER;

    let mut packet = vec![];
    packet.extend_from_slice(&(PROTOCOL_IDENTIFIER as u64).to_be_bytes());
    packet.extend_from_slice(&0u32.to_be_bytes());
    packet.extend_from_slice(&12345u32.to_be_bytes());
    c.bench_function("udp_connect_request_parse", |b| {
        b.iter(|| {
            let _ = std::hint::black_box(Request::from_bytes(&packet[..], 74));
        });
    });
}

fn bench_peer_filtering_ipv4_vs_ipv6(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let tracker = rt.block_on(create_tracker());
    let info_hash = random_info_hash();
    for i in 0..500 {
        let peer_id = random_peer_id();
        let peer = if i % 2 == 0 {
            create_test_peer(IpAddr::V4(Ipv4Addr::new(10, 0, (i / 256) as u8, (i % 256) as u8)), 6881, peer_id)
        } else {
            create_test_peer(IpAddr::V6(format!("2001:db8::{:x}:{:x}", i / 256, i % 256).parse().unwrap()), 6881, peer_id)
        };
        tracker.add_torrent_peer(info_hash, peer_id, peer, false);
    }
    let mut group = c.benchmark_group("peer_filtering");
    group.bench_function("ipv4_only", |b| {
        b.iter(|| {
            std::hint::black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::IPv4, None));
        });
    });
    group.bench_function("ipv6_only", |b| {
        b.iter(|| {
            std::hint::black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::IPv6, None));
        });
    });
    group.bench_function("all_types", |b| {
        b.iter(|| {
            std::hint::black_box(tracker.get_torrent_peers(info_hash, 50, TorrentPeersType::All, None));
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_add_peer,
    bench_announce_into_populated_swarm,
    bench_get_peers_with_limit,
    bench_concurrent_peer_additions,
    bench_contended_stats,
    bench_sharding_distribution,
    bench_parse_query,
    bench_udp_packet_parsing,
    bench_peer_filtering_ipv4_vs_ipv6,
);

criterion_main!(benches);