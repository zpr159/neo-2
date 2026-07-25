#[cfg(test)]
mod tests {
    use neo_core::security::*;
    use neo_core::security::auth::MockAuthenticator;
    use std::collections::HashMap;

    // ── Permission enum ───────────────────────────────────────────────

    #[test]
    fn test_permission_variants_display() {
        assert_eq!(format!("{}", Permission::Read), "read");
        assert_eq!(format!("{}", Permission::Write), "write");
        assert_eq!(format!("{}", Permission::Execute), "execute");
        assert_eq!(format!("{}", Permission::Admin), "admin");
        assert_eq!(format!("{}", Permission::SystemRead), "system_read");
        assert_eq!(format!("{}", Permission::SystemWrite), "system_write");
        assert_eq!(format!("{}", Permission::ToolUse), "tool_use");
        assert_eq!(format!("{}", Permission::WorkflowExecute), "workflow_execute");
        assert_eq!(format!("{}", Permission::AgentControl), "agent_control");
        assert_eq!(format!("{}", Permission::MemoryAccess), "memory_access");
        assert_eq!(format!("{}", Permission::KnowledgeAccess), "knowledge_access");
        assert_eq!(format!("{}", Permission::WorldModelAccess), "world_model_access");
    }

    #[test]
    fn test_permission_serde_roundtrip() {
        let perms = vec![
            Permission::Read,
            Permission::Write,
            Permission::Execute,
            Permission::Admin,
            Permission::SystemRead,
            Permission::SystemWrite,
            Permission::ToolUse,
            Permission::WorkflowExecute,
            Permission::AgentControl,
            Permission::MemoryAccess,
            Permission::KnowledgeAccess,
            Permission::WorldModelAccess,
        ];
        for p in &perms {
            let json = serde_json::to_string(p).unwrap();
            let deserialized: Permission = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, p);
        }
    }

    // ── PermissionSet ─────────────────────────────────────────────────

    #[test]
    fn test_permission_set_new_empty() {
        let ps = PermissionSet::new();
        assert!(!ps.has(&Permission::Read));
        assert!(!ps.is_admin());
    }

    #[test]
    fn test_permission_set_add_and_has() {
        let mut ps = PermissionSet::new();
        ps.add(Permission::Read);
        assert!(ps.has(&Permission::Read));
        assert!(!ps.has(&Permission::Write));
    }

    #[test]
    fn test_permission_set_add_no_duplicates() {
        let mut ps = PermissionSet::new();
        ps.add(Permission::Read);
        ps.add(Permission::Read);
        assert_eq!(ps.permissions.len(), 1);
    }

    #[test]
    fn test_permission_set_remove() {
        let mut ps = PermissionSet::new();
        ps.add(Permission::Read);
        ps.add(Permission::Write);
        ps.remove(&Permission::Read);
        assert!(!ps.has(&Permission::Read));
        assert!(ps.has(&Permission::Write));
    }

    #[test]
    fn test_permission_set_remove_nonexistent() {
        let mut ps = PermissionSet::new();
        ps.remove(&Permission::Read);
        assert!(!ps.has(&Permission::Read));
    }

    #[test]
    fn test_permission_set_is_admin() {
        let mut ps = PermissionSet::new();
        assert!(!ps.is_admin());
        ps.add(Permission::Admin);
        assert!(ps.is_admin());
    }

    #[test]
    fn test_permission_set_default() {
        let ps = PermissionSet::default();
        assert!(ps.permissions.is_empty());
    }

    #[test]
    fn test_permission_set_serde_roundtrip() {
        let mut ps = PermissionSet::new();
        ps.add(Permission::Read);
        ps.add(Permission::Admin);
        let json = serde_json::to_string(&ps).unwrap();
        let deserialized: PermissionSet = serde_json::from_str(&json).unwrap();
        assert!(deserialized.has(&Permission::Read));
        assert!(deserialized.has(&Permission::Admin));
        assert!(!deserialized.has(&Permission::Write));
    }

    // ── PermissionManager ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_permission_manager_new() {
        let pm = PermissionManager::new();
        assert!(!pm.check("user1", &Permission::Read).await);
    }

    #[tokio::test]
    async fn test_permission_manager_grant() {
        let pm = PermissionManager::new();
        pm.grant("user1", Permission::Read).await;
        assert!(pm.check("user1", &Permission::Read).await);
        assert!(!pm.check("user1", &Permission::Write).await);
    }

    #[tokio::test]
    async fn test_permission_manager_revoke() {
        let pm = PermissionManager::new();
        pm.grant("user1", Permission::Read).await;
        pm.grant("user1", Permission::Write).await;
        pm.revoke("user1", &Permission::Read).await;
        assert!(!pm.check("user1", &Permission::Read).await);
        assert!(pm.check("user1", &Permission::Write).await);
    }

    #[tokio::test]
    async fn test_permission_manager_admin_bypasses() {
        let pm = PermissionManager::new();
        pm.grant("user1", Permission::Admin).await;
        assert!(pm.check("user1", &Permission::Read).await);
        assert!(pm.check("user1", &Permission::Write).await);
        assert!(pm.check("user1", &Permission::WorldModelAccess).await);
    }

    #[tokio::test]
    async fn test_permission_manager_list_permissions() {
        let pm = PermissionManager::new();
        pm.grant("user1", Permission::Read).await;
        pm.grant("user1", Permission::Write).await;
        let perms = pm.list_permissions("user1").await;
        assert_eq!(perms.len(), 2);
    }

    #[tokio::test]
    async fn test_permission_manager_list_permissions_unknown_user() {
        let pm = PermissionManager::new();
        let perms = pm.list_permissions("unknown").await;
        assert!(perms.is_empty());
    }

    // ── MockAuthenticator ─────────────────────────────────────────────

    #[test]
    fn test_mock_authenticator_new() {
        let _auth = MockAuthenticator::new();
    }

    #[test]
    fn test_mock_authenticator_default() {
        let _auth = MockAuthenticator::default();
    }

    #[tokio::test]
    async fn test_mock_authenticator_authenticate() {
        let auth = MockAuthenticator::new();
        let result = auth.authenticate("anything").await;
        assert!(result.authenticated);
        assert!(result.token.is_some());
        assert!(result.error.is_none());
        let token = result.token.unwrap();
        assert_eq!(token.user_id, "mock-user");
    }

    #[tokio::test]
    async fn test_mock_authenticator_validate_token() {
        let auth = MockAuthenticator::new();
        let token = AuthToken {
            token_id: "t1".into(),
            user_id: "u1".into(),
            roles: vec![],
            expires_at: String::new(),
            issued_at: String::new(),
            metadata: HashMap::new(),
        };
        assert!(auth.validate_token(&token).await);
    }

    #[tokio::test]
    async fn test_mock_authenticator_revoke_token() {
        let auth = MockAuthenticator::new();
        assert!(auth.revoke_token("t1").await.is_ok());
    }

    #[tokio::test]
    async fn test_mock_authenticator_refresh_token() {
        let auth = MockAuthenticator::new();
        let token = AuthToken {
            token_id: "old-token".into(),
            user_id: "u1".into(),
            roles: vec!["admin".into()],
            expires_at: String::new(),
            issued_at: String::new(),
            metadata: HashMap::new(),
        };
        let result = auth.refresh_token(&token).await;
        assert!(result.authenticated);
        let new_token = result.token.unwrap();
        assert_eq!(new_token.token_id, "old-token-refreshed");
        assert_eq!(new_token.user_id, "u1");
    }

    // ── AuthToken / AuthResult ────────────────────────────────────────

    #[test]
    fn test_auth_token_serde_roundtrip() {
        let token = AuthToken {
            token_id: "t1".into(),
            user_id: "user1".into(),
            roles: vec!["admin".into()],
            expires_at: "2099-12-31T23:59:59Z".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&token).unwrap();
        let deserialized: AuthToken = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token_id, "t1");
        assert_eq!(deserialized.roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_auth_result_success_display() {
        let r = AuthResult {
            authenticated: true,
            token: None,
            error: None,
        };
        assert_eq!(format!("{r}"), "authenticated");
    }

    #[test]
    fn test_auth_result_failure_display() {
        let r = AuthResult {
            authenticated: false,
            token: None,
            error: Some("bad password".into()),
        };
        assert_eq!(format!("{r}"), "authentication failed: bad password");
    }

    #[test]
    fn test_auth_result_failure_no_error() {
        let r = AuthResult {
            authenticated: false,
            token: None,
            error: None,
        };
        assert_eq!(format!("{r}"), "authentication failed");
    }

    // ── InMemoryCredentialStore ───────────────────────────────────────

    #[tokio::test]
    async fn test_credential_store_new() {
        let _store = InMemoryCredentialStore::new("/tmp/test".into());
    }

    #[tokio::test]
    async fn test_credential_store_store_and_retrieve() {
        let mut store = InMemoryCredentialStore::new("".into());
        let cred = Credential {
            id: "cred1".into(),
            name: "API Key".into(),
            credential_type: CredentialType::ApiKey,
            value: "secret123".into(),
            metadata: HashMap::new(),
            created_at: "2026-01-01T00:00:00Z".into(),
            expires_at: None,
        };
        store.store(cred).await;
        let retrieved = store.retrieve("cred1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "secret123");
    }

    #[tokio::test]
    async fn test_credential_store_retrieve_nonexistent() {
        let store = InMemoryCredentialStore::new("".into());
        assert!(store.retrieve("missing").await.is_none());
    }

    #[tokio::test]
    async fn test_credential_store_delete() {
        let mut store = InMemoryCredentialStore::new("".into());
        let cred = Credential {
            id: "cred1".into(),
            name: "key".into(),
            credential_type: CredentialType::Token,
            value: "val".into(),
            metadata: HashMap::new(),
            created_at: "2026-01-01".into(),
            expires_at: None,
        };
        store.store(cred).await;
        assert!(store.delete("cred1").await.is_ok());
        assert!(store.retrieve("cred1").await.is_none());
    }

    #[tokio::test]
    async fn test_credential_store_delete_nonexistent() {
        let mut store = InMemoryCredentialStore::new("".into());
        let result = store.delete("missing").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_credential_store_list() {
        let mut store = InMemoryCredentialStore::new("".into());
        for i in 0..3 {
            store
                .store(Credential {
                    id: format!("c{i}"),
                    name: format!("cred{i}"),
                    credential_type: CredentialType::ApiKey,
                    value: format!("v{i}"),
                    metadata: HashMap::new(),
                    created_at: "2026-01-01".into(),
                    expires_at: None,
                })
                .await;
        }
        let all = store.list().await;
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_credential_store_rotate() {
        let mut store = InMemoryCredentialStore::new("".into());
        store
            .store(Credential {
                id: "c1".into(),
                name: "k".into(),
                credential_type: CredentialType::Password,
                value: "old".into(),
                metadata: HashMap::new(),
                created_at: "2026-01-01".into(),
                expires_at: None,
            })
            .await;
        let result = store.rotate("c1", "new".into()).await;
        assert!(result.is_ok());
        let cred = store.retrieve("c1").await.unwrap();
        assert_eq!(cred.value, "new");
    }

    #[tokio::test]
    async fn test_credential_store_rotate_nonexistent() {
        let mut store = InMemoryCredentialStore::new("".into());
        let result = store.rotate("missing", "new".into()).await;
        assert!(result.is_err());
    }

    // ── CredentialType ────────────────────────────────────────────────

    #[test]
    fn test_credential_type_display() {
        assert_eq!(format!("{}", CredentialType::ApiKey), "api_key");
        assert_eq!(format!("{}", CredentialType::Password), "password");
        assert_eq!(format!("{}", CredentialType::Token), "token");
        assert_eq!(format!("{}", CredentialType::Certificate), "certificate");
        assert_eq!(format!("{}", CredentialType::OAuth2), "oauth2");
    }

    #[test]
    fn test_credential_type_serde_roundtrip() {
        let types = [
            CredentialType::ApiKey,
            CredentialType::Password,
            CredentialType::Token,
            CredentialType::Certificate,
            CredentialType::OAuth2,
        ];
        for ct in &types {
            let json = serde_json::to_string(ct).unwrap();
            let deserialized: CredentialType = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, ct);
        }
    }

    // ── Credential ────────────────────────────────────────────────────

    #[test]
    fn test_credential_serde_roundtrip() {
        let cred = Credential {
            id: "c1".into(),
            name: "key".into(),
            credential_type: CredentialType::OAuth2,
            value: "secret".into(),
            metadata: HashMap::new(),
            created_at: "2026-01-01".into(),
            expires_at: Some("2027-01-01".into()),
        };
        let json = serde_json::to_string(&cred).unwrap();
        let deserialized: Credential = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.credential_type, CredentialType::OAuth2);
        assert!(deserialized.expires_at.is_some());
    }

    // ── PolicyEngine ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_policy_engine_new() {
        let pe = PolicyEngine::new();
        let policies = pe.list_policies().await;
        assert!(policies.is_empty());
    }

    #[tokio::test]
    async fn test_policy_engine_add_policy() {
        let pe = PolicyEngine::new();
        let policy = Policy {
            id: "p1".into(),
            name: "allow read".into(),
            rules: vec![PolicyRule {
                id: "r1".into(),
                description: "allow read".into(),
                subject_pattern: "*".into(),
                resource_pattern: "file1".into(),
                action: "read".into(),
                effect: PolicyEffect::Allow,
                conditions: HashMap::new(),
            }],
            version: "1.0".into(),
        };
        pe.add_policy(policy).await;
        let policies = pe.list_policies().await;
        assert_eq!(policies.len(), 1);
    }

    #[tokio::test]
    async fn test_policy_engine_evaluate_allow() {
        let pe = PolicyEngine::new();
        pe.add_policy(Policy {
            id: "p1".into(),
            name: "test".into(),
            rules: vec![PolicyRule {
                id: "r1".into(),
                description: "".into(),
                subject_pattern: "*".into(),
                resource_pattern: "res1".into(),
                action: "read".into(),
                effect: PolicyEffect::Allow,
                conditions: HashMap::new(),
            }],
            version: "1.0".into(),
        })
        .await;
        let effect = pe.evaluate("user1", "res1", "read").await;
        assert_eq!(effect, PolicyEffect::Allow);
    }

    #[tokio::test]
    async fn test_policy_engine_evaluate_deny_default() {
        let pe = PolicyEngine::new();
        let effect = pe.evaluate("user1", "res1", "read").await;
        assert_eq!(effect, PolicyEffect::Deny);
    }

    #[tokio::test]
    async fn test_policy_engine_check_access() {
        let pe = PolicyEngine::new();
        pe.add_policy(Policy {
            id: "p1".into(),
            name: "test".into(),
            rules: vec![PolicyRule {
                id: "r1".into(),
                description: "".into(),
                subject_pattern: "admin".into(),
                resource_pattern: "*".into(),
                action: "write".into(),
                effect: PolicyEffect::Allow,
                conditions: HashMap::new(),
            }],
            version: "1.0".into(),
        })
        .await;
        assert!(pe.check_access("admin", &[], "any", "write").await);
        assert!(!pe.check_access("user1", &[], "any", "write").await);
    }

    #[tokio::test]
    async fn test_policy_engine_check_access_via_role() {
        let pe = PolicyEngine::new();
        pe.add_policy(Policy {
            id: "p1".into(),
            name: "test".into(),
            rules: vec![PolicyRule {
                id: "r1".into(),
                description: "".into(),
                subject_pattern: "admin".into(),
                resource_pattern: "*".into(),
                action: "delete".into(),
                effect: PolicyEffect::Allow,
                conditions: HashMap::new(),
            }],
            version: "1.0".into(),
        })
        .await;
        let roles = vec!["admin".to_string()];
        assert!(pe.check_access("user1", &roles, "any", "delete").await);
    }

    #[tokio::test]
    async fn test_policy_engine_wildcard_pattern() {
        let pe = PolicyEngine::new();
        pe.add_policy(Policy {
            id: "p1".into(),
            name: "test".into(),
            rules: vec![PolicyRule {
                id: "r1".into(),
                description: "".into(),
                subject_pattern: "*".into(),
                resource_pattern: "file_*".into(),
                action: "read".into(),
                effect: PolicyEffect::Allow,
                conditions: HashMap::new(),
            }],
            version: "1.0".into(),
        })
        .await;
        assert!(pe.evaluate("anyone", "file_1", "read").await == PolicyEffect::Allow);
        assert!(pe.evaluate("anyone", "file_2", "read").await == PolicyEffect::Allow);
        assert!(pe.evaluate("anyone", "other", "read").await == PolicyEffect::Deny);
    }

    // ── PolicyRule / Policy serialization ─────────────────────────────

    #[test]
    fn test_policy_rule_serde_roundtrip() {
        let rule = PolicyRule {
            id: "r1".into(),
            description: "allow read".into(),
            subject_pattern: "user_*".into(),
            resource_pattern: "file_*".into(),
            action: "read".into(),
            effect: PolicyEffect::Allow,
            conditions: HashMap::new(),
        };
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: PolicyRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "r1");
        assert_eq!(deserialized.effect, PolicyEffect::Allow);
    }

    #[test]
    fn test_policy_serde_roundtrip() {
        let policy = Policy {
            id: "p1".into(),
            name: "test".into(),
            rules: vec![],
            version: "1.0".into(),
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: Policy = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.version, "1.0");
    }

    #[test]
    fn test_policy_effect_display() {
        assert_eq!(format!("{}", PolicyEffect::Allow), "allow");
        assert_eq!(format!("{}", PolicyEffect::Deny), "deny");
    }

    #[test]
    fn test_policy_effect_serde_roundtrip() {
        assert_eq!(
            serde_json::from_str::<PolicyEffect>(&serde_json::to_string(&PolicyEffect::Allow).unwrap()).unwrap(),
            PolicyEffect::Allow
        );
        assert_eq!(
            serde_json::from_str::<PolicyEffect>(&serde_json::to_string(&PolicyEffect::Deny).unwrap()).unwrap(),
            PolicyEffect::Deny
        );
    }

    // ── AuditLogger ───────────────────────────────────────────────────

    #[tokio::test]
    async fn test_audit_logger_new() {
        let logger = AuditLogger::new();
        assert_eq!(logger.event_count().await, 0);
    }

    #[tokio::test]
    async fn test_audit_logger_log_event() {
        let logger = AuditLogger::new();
        logger
            .log_event("user1", "read", "file1", AuditOutcome::Success, None)
            .await;
        assert_eq!(logger.event_count().await, 1);
    }

    #[tokio::test]
    async fn test_audit_logger_query_events() {
        let logger = AuditLogger::new();
        logger
            .log_event("user1", "read", "file1", AuditOutcome::Success, None)
            .await;
        logger
            .log_event("user2", "write", "file2", AuditOutcome::Failure, None)
            .await;
        logger
            .log_event("user1", "delete", "file3", AuditOutcome::Denied, Some("no permission".into()))
            .await;
        let events = logger.query_events("user1").await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_audit_logger_query_events_no_match() {
        let logger = AuditLogger::new();
        let events = logger.query_events("nobody").await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_audit_logger_export_events() {
        let logger = AuditLogger::new();
        logger
            .log_event("u1", "a1", "r1", AuditOutcome::Success, None)
            .await;
        logger
            .log_event("u2", "a2", "r2", AuditOutcome::Failure, None)
            .await;
        let all = logger.export_events().await;
        assert_eq!(all.len(), 2);
    }

    // ── AuditEvent ────────────────────────────────────────────────────

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent {
            id: "e1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            user_id: "user1".into(),
            action: "read".into(),
            resource: "file1".into(),
            outcome: AuditOutcome::Success,
            details: None,
            metadata: HashMap::new(),
        };
        assert_eq!(event.user_id, "user1");
        assert_eq!(event.outcome, AuditOutcome::Success);
    }

    #[test]
    fn test_audit_event_serde_roundtrip() {
        let mut meta = HashMap::new();
        meta.insert("key".to_string(), "val".to_string());
        let event = AuditEvent {
            id: "e1".into(),
            timestamp: "2026-01-01T00:00:00Z".into(),
            user_id: "user1".into(),
            action: "write".into(),
            resource: "res1".into(),
            outcome: AuditOutcome::Failure,
            details: Some("error occurred".into()),
            metadata: meta,
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "e1");
        assert_eq!(deserialized.outcome, AuditOutcome::Failure);
        assert_eq!(deserialized.details, Some("error occurred".into()));
    }

    #[test]
    fn test_audit_outcome_display() {
        assert_eq!(format!("{}", AuditOutcome::Success), "success");
        assert_eq!(format!("{}", AuditOutcome::Failure), "failure");
        assert_eq!(format!("{}", AuditOutcome::Denied), "denied");
    }

    // ── EncryptionManager ─────────────────────────────────────────────

    #[test]
    fn test_encryption_manager_new() {
        let em = EncryptionManager::new();
        assert!(em.list_keys().is_empty());
        assert!(em.active_key_id().is_none());
    }

    #[test]
    fn test_encryption_manager_encrypt_decrypt_passthrough() {
        let em = EncryptionManager::new();
        let data = b"hello world";
        let encrypted = em.encrypt(data);
        let decrypted = em.decrypt(&encrypted);
        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_encryption_manager_rotate_key() {
        let mut em = EncryptionManager::new();
        let key = em.rotate_key(EncryptionAlgorithm::Aes256Gcm);
        assert!(key.key_id.starts_with("key-"));
        assert_eq!(key.algorithm, EncryptionAlgorithm::Aes256Gcm);
        assert!(em.active_key_id().is_some());
        assert_eq!(em.list_keys().len(), 1);
    }

    #[test]
    fn test_encryption_manager_rotate_multiple_keys() {
        let mut em = EncryptionManager::new();
        em.rotate_key(EncryptionAlgorithm::Aes256Gcm);
        em.rotate_key(EncryptionAlgorithm::ChaCha20Poly1305);
        assert_eq!(em.list_keys().len(), 2);
        assert_eq!(em.active_key_id(), em.list_keys().last().map(|k| k.key_id.as_str()));
    }

    #[test]
    fn test_encryption_manager_default() {
        let em = EncryptionManager::default();
        assert!(em.list_keys().is_empty());
    }

    // ── EncryptionKey ─────────────────────────────────────────────────

    #[test]
    fn test_encryption_key_serde_roundtrip() {
        let key = EncryptionKey {
            key_id: "key-1".into(),
            algorithm: EncryptionAlgorithm::Ed25519,
            created_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&key).unwrap();
        let deserialized: EncryptionKey = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.algorithm, EncryptionAlgorithm::Ed25519);
    }

    // ── EncryptionAlgorithm ───────────────────────────────────────────

    #[test]
    fn test_encryption_algorithm_display() {
        assert_eq!(format!("{}", EncryptionAlgorithm::Aes256Gcm), "aes-256-gcm");
        assert_eq!(format!("{}", EncryptionAlgorithm::ChaCha20Poly1305), "chacha20-poly1305");
        assert_eq!(format!("{}", EncryptionAlgorithm::Ed25519), "ed25519");
    }

    #[test]
    fn test_encryption_algorithm_serde_roundtrip() {
        let algos = [
            EncryptionAlgorithm::Aes256Gcm,
            EncryptionAlgorithm::ChaCha20Poly1305,
            EncryptionAlgorithm::Ed25519,
        ];
        for a in &algos {
            let json = serde_json::to_string(a).unwrap();
            let deserialized: EncryptionAlgorithm = serde_json::from_str(&json).unwrap();
            assert_eq!(&deserialized, a);
        }
    }

    // ── SecurityConfig ────────────────────────────────────────────────

    #[test]
    fn test_security_config_default() {
        let cfg = SecurityConfig::default();
        assert!(cfg.auth_enabled);
        assert!(!cfg.encryption_enabled);
        assert!(cfg.audit_enabled);
        assert!(cfg.policy_file.is_none());
        assert!(cfg.credential_store_path.is_none());
    }

    #[test]
    fn test_security_config_serde_roundtrip() {
        let cfg = SecurityConfig {
            auth_enabled: false,
            encryption_enabled: true,
            audit_enabled: false,
            policy_file: Some("/etc/policy.json".into()),
            credential_store_path: Some("/var/creds".into()),
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let deserialized: SecurityConfig = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.auth_enabled);
        assert!(deserialized.encryption_enabled);
    }

    // ── SecurityManager ───────────────────────────────────────────────

    #[test]
    fn test_security_manager_new() {
        let sm = SecurityManager::new(SecurityConfig::default());
        assert_eq!(sm.config().auth_enabled, true);
    }

    #[test]
    fn test_security_manager_debug() {
        let sm = SecurityManager::new(SecurityConfig::default());
        let debug_str = format!("{:?}", sm);
        assert!(debug_str.contains("SecurityManager"));
    }

    #[test]
    fn test_security_manager_references() {
        let sm = SecurityManager::new(SecurityConfig::default());
        let _pm = sm.permission_manager();
        let _pe = sm.policy_engine();
        let _al = sm.audit_logger();
        let _em = sm.encryption_manager();
    }

    #[tokio::test]
    async fn test_security_manager_check_permission() {
        let sm = SecurityManager::new(SecurityConfig::default());
        assert!(!sm.check_permission("user1", &Permission::Read).await);
        sm.permission_manager().grant("user1", Permission::Read).await;
        assert!(sm.check_permission("user1", &Permission::Read).await);
    }
}
