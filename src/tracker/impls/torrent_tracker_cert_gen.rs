use std::fs;
use std::process::exit;
use log::{error, info};
use rcgen::{generate_simple_self_signed, CertifiedKey};
use crate::structs::Cli;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

impl TorrentTracker {
    #[tracing::instrument]
    pub async fn cert_gen(&self, args: &Cli)
    {
        info!("[CERTGEN] Requesting to generate a self-signed key and certificate file");

        // Set localhost and optional domain if given.
        let mut subject_alt_names = vec![
            String::from("localhost")
        ];
        if args.selfsigned_domain != *"localhost" {
            subject_alt_names.push(args.selfsigned_domain.clone());
        }

        // Generate X.509 key and cert file.
        let CertifiedKey { cert, key_pair} = generate_simple_self_signed(subject_alt_names).unwrap();

        // Write the key and cert file.
        match fs::write(args.selfsigned_keyfile.as_str(), key_pair.serialize_pem()) {
            Ok(_) => {
                info!("[CERTGEN] The key file {} has been generated", args.selfsigned_keyfile.as_str());
            }
            Err(error) => {
                error!("[CERTGEN] The key file {} could not be generated!", args.selfsigned_keyfile.as_str());
                panic!("[CERTGEN] {}", error)
            }
        }
        match fs::write(args.selfsigned_certfile.as_str(), cert.pem()) {
            Ok(_) => {
                info!("[CERTGEN] The cert file {} has been generated", args.selfsigned_certfile.as_str());
            }
            Err(error) => {
                error!("[CERTGEN] The cert file {} could not be generated!", args.selfsigned_certfile.as_str());
                panic!("[CERTGEN] {}", error)
            }
        }

        info!("[CERTGEN] The files {} and {} has been generated, use them only for development reasons", args.selfsigned_keyfile.as_str(), args.selfsigned_certfile.as_str());
        exit(0)
    }
}