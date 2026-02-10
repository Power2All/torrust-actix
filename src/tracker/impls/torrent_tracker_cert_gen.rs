use crate::structs::Cli;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use log::{
    error,
    info
};
use rcgen::{
    generate_simple_self_signed,
    CertifiedKey
};
use std::fs;
use std::process::exit;

impl TorrentTracker {
    #[tracing::instrument(level = "debug")]
    pub async fn cert_gen(&self, args: &Cli)
    {
        info!("[CERTGEN] Requesting to generate a self-signed key and certificate file");
        let mut subject_alt_names = vec![
            String::from("localhost")
        ];
        if args.selfsigned_domain != "localhost" {
            subject_alt_names.push(args.selfsigned_domain.clone());
        }
        let CertifiedKey { cert, signing_key } = generate_simple_self_signed(subject_alt_names)
            .expect("[CERTGEN] Failed to generate self-signed certificate");
        let keyfile = &args.selfsigned_keyfile;
        let certfile = &args.selfsigned_certfile;
        if let Err(error) = fs::write(keyfile, signing_key.serialize_pem()) {
            error!("[CERTGEN] The key file {keyfile} could not be generated!");
            panic!("[CERTGEN] {error}")
        }
        info!("[CERTGEN] The key file {keyfile} has been generated");
        if let Err(error) = fs::write(certfile, cert.pem()) {
            error!("[CERTGEN] The cert file {certfile} could not be generated!");
            panic!("[CERTGEN] {error}")
        }
        info!("[CERTGEN] The cert file {certfile} has been generated");
        info!("[CERTGEN] The files {keyfile} and {certfile} have been generated, use them only for development reasons");
        exit(0)
    }
}