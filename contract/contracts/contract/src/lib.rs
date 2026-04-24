#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String, Symbol,
};

// ─────────────────────────────────────────────────────────────
//  Storage key types
// ─────────────────────────────────────────────────────────────

/// Top-level storage keys
const ADMIN: Symbol = symbol_short!("ADMIN");

/// Per-credential key prefix  (combined with subject address at runtime)
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Credential(Address),
    Issuer(Address),
}

// ─────────────────────────────────────────────────────────────
//  Data structures
// ─────────────────────────────────────────────────────────────

/// Credit score tiers mapped to a numeric band
#[contracttype]
#[derive(Clone, PartialEq)]
pub enum ScoreTier {
    Poor,        // 300 – 579
    Fair,        // 580 – 669
    Good,        // 670 – 739
    VeryGood,    // 740 – 799
    Exceptional, // 800 – 850
}

/// The portable on-chain credential issued to a subject
#[contracttype]
#[derive(Clone)]
pub struct CreditCredential {
    /// The wallet address this credential belongs to
    pub subject: Address,
    /// Raw score value (300 – 850)
    pub score: u32,
    /// Derived tier based on the score
    pub tier: ScoreTier,
    /// Free-form metadata (e.g. "auto-loan", "mortgage")
    pub context: String,
    /// Ledger timestamp of issuance
    pub issued_at: u64,
    /// Ledger timestamp of last update
    pub updated_at: u64,
    /// Whether this credential is currently active
    pub is_active: bool,
    /// Address of the entity that issued this credential
    pub issuer: Address,
    /// Monotonically increasing version counter
    pub version: u32,
}

// ─────────────────────────────────────────────────────────────
//  Events
// ─────────────────────────────────────────────────────────────

const EVT_ISSUED:   Symbol = symbol_short!("ISSUED");
const EVT_UPDATED:  Symbol = symbol_short!("UPDATED");
const EVT_REVOKED:  Symbol = symbol_short!("REVOKED");
const EVT_VERIFIED: Symbol = symbol_short!("VERIFIED");

// ─────────────────────────────────────────────────────────────
//  Contract
// ─────────────────────────────────────────────────────────────

#[contract]
pub struct CreditScoreContract;

#[contractimpl]
impl CreditScoreContract {

    // ── Initialisation ──────────────────────────────────────

    /// Deploy the contract and designate an administrator.
    /// Must be called exactly once.
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&ADMIN) {
            panic!("already initialized");
        }
        env.storage().instance().set(&ADMIN, &admin);
    }

    // ── Issuer management ───────────────────────────────────

    /// Admin registers a trusted issuer address.
    pub fn add_issuer(env: Env, issuer: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .set(&DataKey::Issuer(issuer), &true);
    }

    /// Admin revokes an issuer's permission to create credentials.
    pub fn remove_issuer(env: Env, issuer: Address) {
        Self::require_admin(&env);
        env.storage()
            .persistent()
            .remove(&DataKey::Issuer(issuer));
    }

    /// Returns whether `issuer` is currently authorised.
    pub fn is_issuer(env: Env, issuer: Address) -> bool {
        env.storage()
            .persistent()
            .get(&DataKey::Issuer(issuer))
            .unwrap_or(false)
    }

    // ── Credential lifecycle ─────────────────────────────────

    /// Issue a new credit-score credential to `subject`.
    ///
    /// # Panics
    /// * Caller is not an authorised issuer.
    /// * `score` is outside [300, 850].
    /// * A credential already exists for this subject.
    pub fn issue_credential(
        env: Env,
        issuer: Address,
        subject: Address,
        score: u32,
        context: String,
    ) -> CreditCredential {
        issuer.require_auth();
        Self::require_issuer(&env, &issuer);
        Self::validate_score(score);

        let key = DataKey::Credential(subject.clone());
        if env.storage().persistent().has(&key) {
            panic!("credential already exists; use update_score");
        }

        let now = env.ledger().timestamp();
        let credential = CreditCredential {
            subject:    subject.clone(),
            score,
            tier:       Self::score_to_tier(score),
            context:    context.clone(),
            issued_at:  now,
            updated_at: now,
            is_active:  true,
            issuer:     issuer.clone(),
            version:    1,
        };

        env.storage().persistent().set(&key, &credential);
        env.events().publish(
            (EVT_ISSUED, subject),
            (score, issuer),
        );

        credential
    }

    /// Update the score on an existing, active credential.
    ///
    /// Only the original issuer may update the score.
    pub fn update_score(
        env: Env,
        issuer: Address,
        subject: Address,
        new_score: u32,
        new_context: String,
    ) -> CreditCredential {
        issuer.require_auth();
        Self::require_issuer(&env, &issuer);
        Self::validate_score(new_score);

        let key = DataKey::Credential(subject.clone());
        let mut cred: CreditCredential = env
            .storage()
            .persistent()
            .get(&key)
            .expect("credential not found");

        if !cred.is_active {
            panic!("credential has been revoked");
        }
        if cred.issuer != issuer {
            panic!("only the original issuer may update this credential");
        }

        cred.score      = new_score;
        cred.tier       = Self::score_to_tier(new_score);
        cred.context    = new_context;
        cred.updated_at = env.ledger().timestamp();
        cred.version   += 1;

        env.storage().persistent().set(&key, &cred);
        env.events().publish(
            (EVT_UPDATED, subject),
            (new_score, cred.version),
        );

        cred
    }

    /// Revoke an existing credential, making it inactive.
    ///
    /// Only the original issuer or the admin may revoke.
    pub fn revoke_credential(env: Env, caller: Address, subject: Address) {
        caller.require_auth();

        let key = DataKey::Credential(subject.clone());
        let mut cred: CreditCredential = env
            .storage()
            .persistent()
            .get(&key)
            .expect("credential not found");

        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        let authorised = caller == cred.issuer || caller == admin;
        if !authorised {
            panic!("only the issuer or admin may revoke this credential");
        }

        cred.is_active  = false;
        cred.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&key, &cred);
        env.events().publish((EVT_REVOKED, subject), caller);
    }

    // ── Queries ──────────────────────────────────────────────

    /// Fetch the full credential for `subject`.
    pub fn get_credential(env: Env, subject: Address) -> CreditCredential {
        env.storage()
            .persistent()
            .get(&DataKey::Credential(subject))
            .expect("credential not found")
    }

    /// Return `true` if `subject` holds an active credential with
    /// a score ≥ `min_score`.
    ///
    /// Emits a `VERIFIED` event so third-party protocols can
    /// react on-chain without storing a copy of the score.
    pub fn verify_score(
        env: Env,
        subject: Address,
        min_score: u32,
    ) -> bool {
        let key = DataKey::Credential(subject.clone());
        let cred: Option<CreditCredential> =
            env.storage().persistent().get(&key);

        let result = match cred {
            Some(c) => c.is_active && c.score >= min_score,
            None    => false,
        };

        env.events().publish(
            (EVT_VERIFIED, subject),
            (min_score, result),
        );
        result
    }

    /// Returns the score tier for `subject` without exposing the
    /// raw numeric score — useful for privacy-preserving checks.
    pub fn get_tier(env: Env, subject: Address) -> ScoreTier {
        let cred: CreditCredential = env
            .storage()
            .persistent()
            .get(&DataKey::Credential(subject))
            .expect("credential not found");

        cred.tier
    }

    // ── Admin helpers ────────────────────────────────────────

    /// Transfer admin rights to a new address.
    pub fn transfer_admin(env: Env, new_admin: Address) {
        Self::require_admin(&env);
        env.storage().instance().set(&ADMIN, &new_admin);
    }

    /// Return the current admin address.
    pub fn get_admin(env: Env) -> Address {
        env.storage().instance().get(&ADMIN).unwrap()
    }

    // ── Internal helpers ─────────────────────────────────────

    fn require_admin(env: &Env) {
        let admin: Address = env.storage().instance().get(&ADMIN).unwrap();
        admin.require_auth();
    }

    fn require_issuer(env: &Env, issuer: &Address) {
        let ok: bool = env
            .storage()
            .persistent()
            .get(&DataKey::Issuer(issuer.clone()))
            .unwrap_or(false);
        if !ok {
            panic!("caller is not an authorised issuer");
        }
    }

    fn validate_score(score: u32) {
        if score < 300 || score > 850 {
            panic!("score must be between 300 and 850");
        }
    }

    fn score_to_tier(score: u32) -> ScoreTier {
        match score {
            300..=579 => ScoreTier::Poor,
            580..=669 => ScoreTier::Fair,
            670..=739 => ScoreTier::Good,
            740..=799 => ScoreTier::VeryGood,
            _          => ScoreTier::Exceptional,
        }
    }
}

// ─────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    fn setup() -> (Env, CreditScoreContractClient<'static>, Address, Address, Address) {
        let env     = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CreditScoreContract);
        let client  = CreditScoreContractClient::new(&env, &contract_id);

        let admin   = Address::generate(&env);
        let issuer  = Address::generate(&env);
        let subject = Address::generate(&env);

        client.initialize(&admin);
        client.add_issuer(&issuer);

        (env, client, admin, issuer, subject)
    }

    #[test]
    fn test_issue_and_fetch() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "mortgage");

        let cred = client.issue_credential(&issuer, &subject, &720, &ctx);
        assert_eq!(cred.score, 720);
        assert_eq!(cred.tier,  ScoreTier::Good);
        assert!(cred.is_active);
        assert_eq!(cred.version, 1);
    }

    #[test]
    fn test_update_score() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "auto-loan");

        client.issue_credential(&issuer, &subject, &650, &ctx);

        let updated_ctx = String::from_str(&env, "auto-loan-updated");
        let cred = client.update_score(&issuer, &subject, &780, &updated_ctx);
        assert_eq!(cred.score,   780);
        assert_eq!(cred.tier,    ScoreTier::VeryGood);
        assert_eq!(cred.version, 2);
    }

    #[test]
    fn test_revoke_credential() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "personal");

        client.issue_credential(&issuer, &subject, &700, &ctx);
        client.revoke_credential(&issuer, &subject);

        let cred = client.get_credential(&subject);
        assert!(!cred.is_active);
    }

    #[test]
    fn test_verify_score_pass() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "defi");

        client.issue_credential(&issuer, &subject, &800, &ctx);
        assert!(client.verify_score(&subject, &750));
    }

    #[test]
    fn test_verify_score_fail() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "defi");

        client.issue_credential(&issuer, &subject, &600, &ctx);
        assert!(!client.verify_score(&subject, &750));
    }

    #[test]
    #[should_panic(expected = "score must be between 300 and 850")]
    fn test_invalid_score_panics() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "bad");
        client.issue_credential(&issuer, &subject, &1000, &ctx);
    }

    #[test]
    fn test_get_tier() {
        let (env, client, _admin, issuer, subject) = setup();
        let ctx = String::from_str(&env, "tier-test");

        client.issue_credential(&issuer, &subject, &830, &ctx);
        let tier = client.get_tier(&subject);
        assert_eq!(tier, ScoreTier::Exceptional);
    }
}