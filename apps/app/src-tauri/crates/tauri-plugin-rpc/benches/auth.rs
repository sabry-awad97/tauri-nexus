//! Benchmarks for authentication and authorization
//!
//! Run with: cargo bench --package tauri-plugin-rpc --bench auth
//!
//! These benchmarks measure the performance of:
//! - Pattern matching (Wildcard, Exact, Prefix)
//! - Authorization checks with various configurations
//! - Rule lookup with different numbers of rules
//! - Config creation

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use tauri_plugin_rpc::auth::rules::CompiledPattern;
use tauri_plugin_rpc::auth::{AuthConfig, AuthResult, AuthRule};

// =============================================================================
// Pattern Matching Benchmarks
// =============================================================================

fn bench_pattern_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("pattern_matching");

    // Wildcard pattern
    let wildcard = CompiledPattern::compile("*");
    group.bench_function("wildcard_match", |b| {
        b.iter(|| wildcard.matches(black_box("user.get")))
    });

    // Exact pattern - match
    let exact = CompiledPattern::compile("user.get");
    group.bench_function("exact_match_hit", |b| {
        b.iter(|| exact.matches(black_box("user.get")))
    });

    // Exact pattern - no match
    group.bench_function("exact_match_miss", |b| {
        b.iter(|| exact.matches(black_box("admin.get")))
    });

    // Prefix pattern - match (exact)
    let prefix = CompiledPattern::compile("user.*");
    group.bench_function("prefix_match_exact", |b| {
        b.iter(|| prefix.matches(black_box("user")))
    });

    // Prefix pattern - match (with suffix)
    group.bench_function("prefix_match_suffix", |b| {
        b.iter(|| prefix.matches(black_box("user.get")))
    });

    // Prefix pattern - no match
    group.bench_function("prefix_match_miss", |b| {
        b.iter(|| prefix.matches(black_box("admin.get")))
    });

    group.finish();
}

// =============================================================================
// Authorization Benchmarks
// =============================================================================

fn bench_authorization(c: &mut Criterion) {
    let mut group = c.benchmark_group("authorization");

    // Simple config: public endpoint
    let config_public = AuthConfig::new().public("health");
    let unauth = AuthResult::unauthenticated();

    group.bench_function("public_endpoint_unauth", |b| {
        b.iter(|| config_public.is_authorized(black_box("health"), black_box(&unauth)))
    });

    // Simple config: requires auth
    let config_auth = AuthConfig::new().requires_auth("user.*");
    let auth = AuthResult::authenticated("user-123");

    group.bench_function("requires_auth_authenticated", |b| {
        b.iter(|| config_auth.is_authorized(black_box("user.get"), black_box(&auth)))
    });

    group.bench_function("requires_auth_unauthenticated", |b| {
        b.iter(|| config_auth.is_authorized(black_box("user.get"), black_box(&unauth)))
    });

    // Role-based config
    let config_roles = AuthConfig::new().requires_roles("admin.*", vec!["admin"]);
    let admin = AuthResult::authenticated("admin-123").with_roles(vec!["admin"]);
    let user = AuthResult::authenticated("user-123").with_roles(vec!["user"]);

    group.bench_function("requires_roles_has_role", |b| {
        b.iter(|| config_roles.is_authorized(black_box("admin.users"), black_box(&admin)))
    });

    group.bench_function("requires_roles_lacks_role", |b| {
        b.iter(|| config_roles.is_authorized(black_box("admin.users"), black_box(&user)))
    });

    // Complex config with multiple rules
    let config_complex = AuthConfig::new()
        .public("health")
        .public("auth.login")
        .requires_auth("user.*")
        .requires_roles("admin.*", vec!["admin"])
        .requires_roles("moderator.*", vec!["admin", "moderator"]);

    group.bench_function("complex_config_public", |b| {
        b.iter(|| config_complex.is_authorized(black_box("health"), black_box(&unauth)))
    });

    group.bench_function("complex_config_user", |b| {
        b.iter(|| config_complex.is_authorized(black_box("user.profile"), black_box(&auth)))
    });

    group.bench_function("complex_config_admin", |b| {
        b.iter(|| config_complex.is_authorized(black_box("admin.users"), black_box(&admin)))
    });

    group.finish();
}

// =============================================================================
// Rule Lookup Benchmarks
// =============================================================================

fn bench_rule_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("rule_lookup");

    let auth = AuthResult::authenticated("user-123").with_roles(vec!["user"]);

    // Benchmark with different numbers of rules
    for num_rules in [1, 5, 10, 25, 50, 100].iter() {
        let mut config = AuthConfig::new();

        // Add many rules
        for i in 0..*num_rules {
            config = config.requires_auth(format!("endpoint{}", i));
        }

        // Test first rule (best case)
        group.bench_with_input(
            BenchmarkId::new("first_rule", num_rules),
            num_rules,
            |b, _| b.iter(|| config.is_authorized(black_box("endpoint0"), black_box(&auth))),
        );

        // Test last rule (worst case)
        group.bench_with_input(
            BenchmarkId::new("last_rule", num_rules),
            num_rules,
            |b, _| {
                b.iter(|| {
                    config.is_authorized(
                        black_box(&format!("endpoint{}", num_rules - 1)),
                        black_box(&auth),
                    )
                })
            },
        );

        // Test no match (worst case)
        group.bench_with_input(
            BenchmarkId::new("no_match", num_rules),
            num_rules,
            |b, _| b.iter(|| config.is_authorized(black_box("nonexistent"), black_box(&auth))),
        );
    }

    group.finish();
}

// =============================================================================
// Config Creation Benchmarks
// =============================================================================

fn bench_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_creation");

    // Simple config
    group.bench_function("simple_config", |b| {
        b.iter(|| {
            black_box(
                AuthConfig::new()
                    .public("health")
                    .requires_auth("user.*")
                    .requires_roles("admin.*", vec!["admin"]),
            )
        })
    });

    // Complex config with many rules
    group.bench_function("complex_config_10_rules", |b| {
        b.iter(|| {
            let mut config = AuthConfig::new();
            for i in 0..10 {
                config = config.requires_auth(format!("endpoint{}", i));
            }
            black_box(config)
        })
    });

    group.bench_function("complex_config_50_rules", |b| {
        b.iter(|| {
            let mut config = AuthConfig::new();
            for i in 0..50 {
                config = config.requires_auth(format!("endpoint{}", i));
            }
            black_box(config)
        })
    });

    group.finish();
}

// =============================================================================
// Role Checking Benchmarks
// =============================================================================

fn bench_role_checking(c: &mut Criterion) {
    let mut group = c.benchmark_group("role_checking");

    // Single role
    let single_role = AuthResult::authenticated("user-123").with_roles(vec!["user"]);

    group.bench_function("has_role_single_hit", |b| {
        b.iter(|| single_role.has_role(black_box("user")))
    });

    group.bench_function("has_role_single_miss", |b| {
        b.iter(|| single_role.has_role(black_box("admin")))
    });

    // Multiple roles
    let multi_roles =
        AuthResult::authenticated("user-123").with_roles(vec!["user", "moderator", "premium"]);

    group.bench_function("has_role_multi_first", |b| {
        b.iter(|| multi_roles.has_role(black_box("user")))
    });

    group.bench_function("has_role_multi_last", |b| {
        b.iter(|| multi_roles.has_role(black_box("premium")))
    });

    group.bench_function("has_role_multi_miss", |b| {
        b.iter(|| multi_roles.has_role(black_box("admin")))
    });

    // has_any_role
    group.bench_function("has_any_role_hit", |b| {
        b.iter(|| multi_roles.has_any_role(black_box(&["admin", "moderator"])))
    });

    group.bench_function("has_any_role_miss", |b| {
        b.iter(|| multi_roles.has_any_role(black_box(&["admin", "superuser"])))
    });

    // has_all_roles
    group.bench_function("has_all_roles_hit", |b| {
        b.iter(|| multi_roles.has_all_roles(black_box(&["user", "moderator"])))
    });

    group.bench_function("has_all_roles_miss", |b| {
        b.iter(|| multi_roles.has_all_roles(black_box(&["user", "admin"])))
    });

    group.finish();
}

// =============================================================================
// AuthRule Benchmarks
// =============================================================================

fn bench_auth_rule(c: &mut Criterion) {
    let mut group = c.benchmark_group("auth_rule");

    let auth = AuthResult::authenticated("user-123").with_roles(vec!["user"]);
    let admin = AuthResult::authenticated("admin-123").with_roles(vec!["admin"]);
    let unauth = AuthResult::unauthenticated();

    // Public rule
    let public_rule = AuthRule::public("health");
    group.bench_function("public_rule_satisfied", |b| {
        b.iter(|| public_rule.is_satisfied_by(black_box(&unauth)))
    });

    // Auth required rule
    let auth_rule = AuthRule::requires_auth("user.*");
    group.bench_function("auth_rule_satisfied", |b| {
        b.iter(|| auth_rule.is_satisfied_by(black_box(&auth)))
    });

    group.bench_function("auth_rule_not_satisfied", |b| {
        b.iter(|| auth_rule.is_satisfied_by(black_box(&unauth)))
    });

    // Role required rule
    let role_rule = AuthRule::requires_roles("admin.*", vec!["admin"]);
    group.bench_function("role_rule_satisfied", |b| {
        b.iter(|| role_rule.is_satisfied_by(black_box(&admin)))
    });

    group.bench_function("role_rule_not_satisfied", |b| {
        b.iter(|| role_rule.is_satisfied_by(black_box(&auth)))
    });

    // Rule matching
    group.bench_function("rule_matches_hit", |b| {
        b.iter(|| auth_rule.matches(black_box("user.get")))
    });

    group.bench_function("rule_matches_miss", |b| {
        b.iter(|| auth_rule.matches(black_box("admin.get")))
    });

    group.finish();
}

// =============================================================================
// Criterion Configuration
// =============================================================================

criterion_group!(
    benches,
    bench_pattern_matching,
    bench_authorization,
    bench_rule_lookup,
    bench_config_creation,
    bench_role_checking,
    bench_auth_rule,
);

criterion_main!(benches);
