use seeder::web::api::verify_password;

#[test]
fn plaintext_match() {
    assert!(verify_password("hunter2", "hunter2"));
}

#[test]
fn plaintext_mismatch() {
    assert!(!verify_password("Hunter2", "hunter2"));
}

#[test]
fn plaintext_empty_both() {
    assert!(verify_password("", ""));
}

#[test]
fn plaintext_empty_input() {
    assert!(!verify_password("", "nonempty"));
}

#[test]
fn plaintext_empty_stored() {
    assert!(!verify_password("nonempty", ""));
}

#[test]
fn plaintext_special_chars() {
    let pw = "p@$$w0rd!#%&*()";
    assert!(verify_password(pw, pw));
}

fn make_hash(password: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

#[test]
fn argon2_correct_password() {
    let hash = make_hash("mysecret");
    assert!(verify_password("mysecret", &hash));
}

#[test]
fn argon2_wrong_password() {
    let hash = make_hash("mysecret");
    assert!(!verify_password("wrongpass", &hash));
}

#[test]
fn argon2_empty_password() {
    let hash = make_hash("");
    assert!(verify_password("", &hash));
    assert!(!verify_password("notempty", &hash));
}

#[test]
fn argon2_long_password() {
    let pw = "x".repeat(256);
    let hash = make_hash(&pw);
    assert!(verify_password(&pw, &hash));
    assert!(!verify_password("x", &hash));
}

#[test]
fn argon2_special_chars() {
    let pw = r#"pass"word\n\t with $pecial chars!"#;
    let hash = make_hash(pw);
    assert!(verify_password(pw, &hash));
}

#[test]
fn argon2_malformed_hash_returns_false() {
    assert!(!verify_password("anything", "$argon2id$v=19$m=19456,t=2,p=1$INVALID"));
}

#[test]
fn argon2_completely_garbled_hash() {
    assert!(!verify_password("pw", "$argon2???garbled_data!!!"));
}

#[test]
fn different_hashes_for_same_password() {
    let h1 = make_hash("same");
    let h2 = make_hash("same");
    assert_ne!(h1, h2);
    assert!(verify_password("same", &h1));
    assert!(verify_password("same", &h2));
}